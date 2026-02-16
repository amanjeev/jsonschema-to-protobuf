use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    #[serde(rename = "$id")]
    pub id: Option<String>,

    #[serde(rename = "$schema")]
    pub schema_version: Option<String>,

    #[serde(rename = "$ref")]
    pub reference: Option<String>,

    pub title: Option<String>,
    pub description: Option<String>,

    #[serde(rename = "type")]
    pub schema_type: Option<SchemaType>,

    pub properties: Option<HashMap<String, Schema>>,
    pub required: Option<Vec<String>>,

    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<Box<AdditionalProperties>>,

    pub items: Option<Box<Schema>>,

    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,

    #[serde(rename = "const")]
    pub const_value: Option<serde_json::Value>,

    pub format: Option<String>,

    #[serde(rename = "oneOf")]
    pub one_of: Option<Vec<Schema>>,

    #[serde(rename = "anyOf")]
    pub any_of: Option<Vec<Schema>>,

    #[serde(rename = "allOf")]
    pub all_of: Option<Vec<Schema>>,

    #[serde(rename = "$defs")]
    pub defs: Option<HashMap<String, Schema>>,

    pub definitions: Option<HashMap<String, Schema>>,

    pub default: Option<serde_json::Value>,

    // Validation keywords (captured but not converted to proto)
    #[serde(rename = "minLength")]
    pub min_length: Option<u64>,
    #[serde(rename = "maxLength")]
    pub max_length: Option<u64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub pattern: Option<String>,
    #[serde(rename = "minItems")]
    pub min_items: Option<u64>,
    #[serde(rename = "maxItems")]
    pub max_items: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SchemaType {
    Single(String),
    Multiple(Vec<String>),
}

impl SchemaType {
    pub fn as_single(&self) -> Option<&str> {
        match self {
            SchemaType::Single(s) => Some(s),
            SchemaType::Multiple(v) if v.len() == 1 => Some(&v[0]),
            _ => None,
        }
    }

    pub fn contains(&self, t: &str) -> bool {
        match self {
            SchemaType::Single(s) => s == t,
            SchemaType::Multiple(v) => v.iter().any(|s| s == t),
        }
    }

    pub fn is_nullable(&self) -> bool {
        self.contains("null")
    }

    pub fn non_null_type(&self) -> Option<&str> {
        match self {
            SchemaType::Single(s) if s != "null" => Some(s),
            SchemaType::Multiple(v) => v.iter().find(|s| s.as_str() != "null").map(|s| s.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AdditionalProperties {
    Bool(bool),
    Schema(Schema),
}

impl Schema {
    pub fn parse(json: &str) -> crate::error::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    pub fn get_definitions(&self) -> HashMap<String, &Schema> {
        let mut defs = HashMap::new();

        if let Some(d) = &self.defs {
            for (k, v) in d {
                defs.insert(k.clone(), v);
            }
        }

        if let Some(d) = &self.definitions {
            for (k, v) in d {
                defs.insert(k.clone(), v);
            }
        }

        defs
    }

    pub fn resolve_ref<'a>(&self, root: &'a Schema) -> Option<&'a Schema> {
        let reference = self.reference.as_ref()?;

        // Handle local references like "#/$defs/Foo" or "#/definitions/Foo"
        if reference.starts_with("#/") {
            let path = &reference[2..];
            let parts: Vec<&str> = path.split('/').collect();

            if parts.len() == 2 {
                let (container, name) = (parts[0], parts[1]);
                match container {
                    "$defs" => return root.defs.as_ref()?.get(name),
                    "definitions" => return root.definitions.as_ref()?.get(name),
                    _ => return None,
                }
            }
        }

        None
    }
}
