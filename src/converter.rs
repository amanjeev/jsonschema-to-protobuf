use crate::error::{ConversionError, Result};
use crate::proto::{Enum, EnumValue, Field, Message, OneOf, ProtoFile, ProtoType};
use crate::schema::{AdditionalProperties, Schema, SchemaType};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub package: Option<String>,
}

pub struct Converter {
    options: Options,
    field_counter: u32,
    generated_names: HashSet<String>,
}

impl Converter {
    pub fn new(options: Options) -> Self {
        Self {
            options,
            field_counter: 0,
            generated_names: HashSet::new(),
        }
    }

    pub fn convert(&mut self, schema: &Schema) -> Result<ProtoFile> {
        let mut proto = ProtoFile::new();

        if let Some(pkg) = &self.options.package {
            proto.package = Some(pkg.clone());
        }

        // First, convert all definitions to top-level messages
        let definitions = schema.get_definitions();
        for (name, def_schema) in &definitions {
            let message = self.schema_to_message(name, def_schema, schema)?;
            proto.messages.push(message);
        }

        // Convert the root schema if it has properties
        if schema.properties.is_some() || schema.all_of.is_some() {
            let root_name = schema
                .title
                .clone()
                .unwrap_or_else(|| "Root".to_string());
            let message = self.schema_to_message(&root_name, schema, schema)?;
            proto.messages.push(message);
        }

        Ok(proto)
    }

    fn schema_to_message(&mut self, name: &str, schema: &Schema, root: &Schema) -> Result<Message> {
        let mut message = Message {
            name: to_pascal_case(name),
            comments: schema.description.clone(),
            ..Default::default()
        };

        // Save and reset field counter for this message scope
        let saved_counter = self.field_counter;
        self.field_counter = 0;

        // Handle allOf by merging properties
        let merged_schema = if let Some(all_of) = &schema.all_of {
            self.merge_all_of(all_of, root)?
        } else {
            schema.clone()
        };

        // Convert properties to fields
        if let Some(props) = &merged_schema.properties {
            let required = merged_schema.required.as_ref();

            for (prop_name, prop_schema) in props {
                let is_required = required.map_or(false, |r| r.contains(prop_name));
                self.field_counter += 1;

                let field = self.property_to_field(
                    prop_name,
                    prop_schema,
                    self.field_counter,
                    !is_required,
                    root,
                    &mut message,
                )?;

                message.fields.push(field);
            }
        }

        // Handle oneOf
        if let Some(one_of) = &schema.one_of {
            let oneof = self.convert_one_of("value", one_of, root, &mut message)?;
            message.oneofs.push(oneof);
        }

        // Handle anyOf (treat like oneOf with a warning in comments)
        if let Some(any_of) = &schema.any_of {
            let mut oneof = self.convert_one_of("value", any_of, root, &mut message)?;
            oneof.comments = Some("Note: Converted from anyOf - multiple values may be valid in JSON".to_string());
            message.oneofs.push(oneof);
        }

        // Handle additionalProperties (map type)
        if let Some(additional) = &merged_schema.additional_properties {
            if let AdditionalProperties::Schema(value_schema) = additional.as_ref() {
                self.field_counter += 1;
                let value_type = self.schema_to_proto_type(value_schema, root, &mut message)?;
                let field = Field {
                    name: "additional_properties".to_string(),
                    proto_type: ProtoType::Map {
                        key: Box::new(ProtoType::String),
                        value: Box::new(value_type),
                    },
                    number: self.field_counter,
                    repeated: false,
                    optional: false,
                    comments: Some("Additional properties".to_string()),
                };
                message.fields.push(field);
            }
        }

        // Restore field counter for parent scope
        self.field_counter = saved_counter;

        Ok(message)
    }

    fn merge_all_of(&self, all_of: &[Schema], root: &Schema) -> Result<Schema> {
        let mut merged = Schema::default();
        let mut all_properties: HashMap<String, Schema> = HashMap::new();
        let mut all_required: Vec<String> = Vec::new();

        for schema in all_of {
            // Resolve $ref if present
            let resolved = if schema.reference.is_some() {
                schema.resolve_ref(root).ok_or_else(|| {
                    ConversionError::InvalidRef(schema.reference.clone().unwrap_or_default())
                })?
            } else {
                schema
            };

            if let Some(props) = &resolved.properties {
                for (k, v) in props {
                    all_properties.insert(k.clone(), v.clone());
                }
            }

            if let Some(req) = &resolved.required {
                all_required.extend(req.clone());
            }

            if merged.description.is_none() {
                merged.description = resolved.description.clone();
            }
        }

        merged.properties = Some(all_properties);
        merged.required = Some(all_required);

        Ok(merged)
    }

    fn convert_one_of(
        &mut self,
        name: &str,
        variants: &[Schema],
        root: &Schema,
        parent_message: &mut Message,
    ) -> Result<OneOf> {
        let mut oneof = OneOf {
            name: to_snake_case(name),
            fields: Vec::new(),
            comments: None,
        };

        for (i, variant) in variants.iter().enumerate() {
            self.field_counter += 1;
            let field_number = self.field_counter; // Capture before nested processing

            // Get variant name from $ref or title
            let variant_name = variant
                .reference
                .as_ref()
                .and_then(|r| r.rsplit('/').next().map(|s| s.to_string()))
                .or_else(|| variant.title.clone())
                .unwrap_or_else(|| format!("option_{}", i + 1));

            // Pass original variant to preserve $ref handling
            let proto_type = self.schema_to_proto_type(variant, root, parent_message)?;

            // Get description from resolved schema if available
            let description = if variant.reference.is_some() {
                variant.resolve_ref(root).and_then(|s| s.description.clone())
            } else {
                variant.description.clone()
            };

            oneof.fields.push(Field {
                name: to_snake_case(&variant_name),
                proto_type,
                number: field_number,
                repeated: false,
                optional: false,
                comments: description,
            });
        }

        Ok(oneof)
    }

    fn property_to_field(
        &mut self,
        name: &str,
        schema: &Schema,
        number: u32,
        optional: bool,
        root: &Schema,
        parent_message: &mut Message,
    ) -> Result<Field> {
        // For type checking, resolve $ref if present
        let resolved = if schema.reference.is_some() {
            schema.resolve_ref(root).ok_or_else(|| {
                ConversionError::InvalidRef(schema.reference.clone().unwrap_or_default())
            })?
        } else {
            schema
        };

        // Check if it's an array
        let (proto_type, repeated) = if let Some(SchemaType::Single(t)) = &resolved.schema_type {
            if t == "array" {
                if let Some(items) = &resolved.items {
                    (self.schema_to_proto_type(items, root, parent_message)?, true)
                } else {
                    (ProtoType::Any, true)
                }
            } else {
                // Pass original schema to preserve $ref handling
                (self.schema_to_proto_type(schema, root, parent_message)?, false)
            }
        } else {
            // Pass original schema to preserve $ref handling
            (self.schema_to_proto_type(schema, root, parent_message)?, false)
        };

        // Check for nullable types
        let is_nullable = resolved
            .schema_type
            .as_ref()
            .map_or(false, |t| t.is_nullable());

        Ok(Field {
            name: to_snake_case(name),
            proto_type,
            number,
            repeated,
            optional: optional || is_nullable,
            comments: resolved.description.clone(),
        })
    }

    fn schema_to_proto_type(
        &mut self,
        schema: &Schema,
        root: &Schema,
        parent_message: &mut Message,
    ) -> Result<ProtoType> {
        // Handle $ref
        if let Some(reference) = &schema.reference {
            let ref_name = reference
                .rsplit('/')
                .next()
                .ok_or_else(|| ConversionError::InvalidRef(reference.clone()))?;
            return Ok(ProtoType::Message(to_pascal_case(ref_name)));
        }

        // Handle enum
        if let Some(enum_values) = &schema.enum_values {
            let enum_name = schema
                .title
                .clone()
                .unwrap_or_else(|| self.generate_unique_name("Enum"));

            let proto_enum = self.create_enum(&enum_name, enum_values)?;
            parent_message.nested_enums.push(proto_enum);

            return Ok(ProtoType::Enum(to_pascal_case(&enum_name)));
        }

        // Handle oneOf inline (create nested message)
        if let Some(one_of) = &schema.one_of {
            let msg_name = schema
                .title
                .clone()
                .unwrap_or_else(|| self.generate_unique_name("OneOfValue"));

            let mut nested_msg = Message {
                name: to_pascal_case(&msg_name),
                ..Default::default()
            };

            let oneof = self.convert_one_of("value", one_of, root, &mut nested_msg)?;
            nested_msg.oneofs.push(oneof);
            parent_message.nested_messages.push(nested_msg.clone());

            return Ok(ProtoType::Message(nested_msg.name));
        }

        // Get the type
        let type_str = schema
            .schema_type
            .as_ref()
            .and_then(|t| t.non_null_type())
            .or_else(|| schema.schema_type.as_ref().and_then(|t| t.as_single()));

        match type_str {
            Some("string") => {
                // Check format
                match schema.format.as_deref() {
                    Some("date-time") => Ok(ProtoType::Timestamp),
                    Some("byte") | Some("binary") => Ok(ProtoType::Bytes),
                    _ => Ok(ProtoType::String),
                }
            }
            Some("integer") => Ok(ProtoType::Int64),
            Some("number") => Ok(ProtoType::Double),
            Some("boolean") => Ok(ProtoType::Bool),
            Some("array") => {
                if let Some(items) = &schema.items {
                    self.schema_to_proto_type(items, root, parent_message)
                } else {
                    Ok(ProtoType::Any)
                }
            }
            Some("object") => {
                // Check for additionalProperties (map type)
                if let Some(AdditionalProperties::Schema(value_schema)) = schema
                    .additional_properties
                    .as_ref()
                    .map(|ap| ap.as_ref())
                {
                    let value_type = self.schema_to_proto_type(value_schema, root, parent_message)?;
                    return Ok(ProtoType::Map {
                        key: Box::new(ProtoType::String),
                        value: Box::new(value_type),
                    });
                }

                // Create nested message for object with properties
                if schema.properties.is_some() {
                    let msg_name = schema
                        .title
                        .clone()
                        .unwrap_or_else(|| self.generate_unique_name("Nested"));

                    let nested_msg = self.schema_to_message(&msg_name, schema, root)?;
                    let name = nested_msg.name.clone();
                    parent_message.nested_messages.push(nested_msg);

                    return Ok(ProtoType::Message(name));
                }

                // Generic object without schema
                Ok(ProtoType::Any)
            }
            Some("null") => Ok(ProtoType::Any), // null type maps to Any or could be omitted
            None => {
                // No type specified, check if it's an object by presence of properties
                if schema.properties.is_some() {
                    let msg_name = schema
                        .title
                        .clone()
                        .unwrap_or_else(|| self.generate_unique_name("Nested"));

                    let nested_msg = self.schema_to_message(&msg_name, schema, root)?;
                    let name = nested_msg.name.clone();
                    parent_message.nested_messages.push(nested_msg);

                    Ok(ProtoType::Message(name))
                } else {
                    Ok(ProtoType::Any)
                }
            }
            Some(other) => Err(ConversionError::UnsupportedFeature(format!(
                "Unknown type: {}",
                other
            ))),
        }
    }

    fn create_enum(&self, name: &str, values: &[serde_json::Value]) -> Result<Enum> {
        let mut proto_enum = Enum {
            name: to_pascal_case(name),
            values: Vec::new(),
            comments: None,
        };

        // Proto3 requires first enum value to be 0
        let enum_prefix = to_screaming_snake_case(name);
        proto_enum.values.push(EnumValue {
            name: format!("{}_UNSPECIFIED", enum_prefix),
            number: 0,
        });

        for (i, value) in values.iter().enumerate() {
            let value_name = match value {
                serde_json::Value::String(s) => to_screaming_snake_case(s),
                serde_json::Value::Number(n) => format!("VALUE_{}", n),
                _ => format!("VALUE_{}", i + 1),
            };

            proto_enum.values.push(EnumValue {
                name: format!("{}_{}", enum_prefix, value_name),
                number: (i + 1) as i32,
            });
        }

        Ok(proto_enum)
    }

    fn generate_unique_name(&mut self, prefix: &str) -> String {
        let mut counter = 1;
        loop {
            let name = format!("{}{}", prefix, counter);
            if !self.generated_names.contains(&name) {
                self.generated_names.insert(name.clone());
                return name;
            }
            counter += 1;
        }
    }
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();

    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else if c == '-' || c == ' ' {
            result.push('_');
        } else {
            result.push(c);
        }
    }

    result
}

fn to_screaming_snake_case(s: &str) -> String {
    to_snake_case(s).to_uppercase()
}

pub fn convert(json: &str, options: Options) -> Result<ProtoFile> {
    let schema = Schema::parse(json)?;
    let mut converter = Converter::new(options);
    converter.convert(&schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(to_pascal_case("hello-world"), "HelloWorld");
        assert_eq!(to_pascal_case("helloWorld"), "HelloWorld");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(to_snake_case("helloWorld"), "hello_world");
        assert_eq!(to_snake_case("hello-world"), "hello_world");
    }

    #[test]
    fn test_simple_schema() {
        let schema = r#"{
            "title": "Person",
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            },
            "required": ["name"]
        }"#;

        let result = convert(schema, Options::default()).unwrap();
        let output = result.to_string();

        assert!(output.contains("message Person"));
        assert!(output.contains("string name"));
        assert!(output.contains("int64 age"));
    }
}
