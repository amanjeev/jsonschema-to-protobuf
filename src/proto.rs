use std::fmt;

#[derive(Debug, Clone, Default)]
pub struct ProtoFile {
    pub syntax: String,
    pub package: Option<String>,
    pub imports: Vec<String>,
    pub messages: Vec<Message>,
    pub enums: Vec<Enum>,
}

#[derive(Debug, Clone, Default)]
pub struct Message {
    pub name: String,
    pub fields: Vec<Field>,
    pub oneofs: Vec<OneOf>,
    pub nested_messages: Vec<Message>,
    pub nested_enums: Vec<Enum>,
    pub comments: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub proto_type: ProtoType,
    pub number: u32,
    pub repeated: bool,
    pub optional: bool,
    pub comments: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OneOf {
    pub name: String,
    pub fields: Vec<Field>,
    pub comments: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Enum {
    pub name: String,
    pub values: Vec<EnumValue>,
    pub comments: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EnumValue {
    pub name: String,
    pub number: i32,
}

#[derive(Debug, Clone)]
pub enum ProtoType {
    Double,
    Float,
    Int32,
    Int64,
    UInt32,
    UInt64,
    Bool,
    String,
    Bytes,
    Message(String),
    Enum(String),
    Map { key: Box<ProtoType>, value: Box<ProtoType> },
    Timestamp,
    Any,
}

impl fmt::Display for ProtoType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtoType::Double => write!(f, "double"),
            ProtoType::Float => write!(f, "float"),
            ProtoType::Int32 => write!(f, "int32"),
            ProtoType::Int64 => write!(f, "int64"),
            ProtoType::UInt32 => write!(f, "uint32"),
            ProtoType::UInt64 => write!(f, "uint64"),
            ProtoType::Bool => write!(f, "bool"),
            ProtoType::String => write!(f, "string"),
            ProtoType::Bytes => write!(f, "bytes"),
            ProtoType::Message(name) => write!(f, "{}", name),
            ProtoType::Enum(name) => write!(f, "{}", name),
            ProtoType::Map { key, value } => write!(f, "map<{}, {}>", key, value),
            ProtoType::Timestamp => write!(f, "google.protobuf.Timestamp"),
            ProtoType::Any => write!(f, "google.protobuf.Any"),
        }
    }
}

impl ProtoFile {
    pub fn new() -> Self {
        Self {
            syntax: "proto3".to_string(),
            ..Default::default()
        }
    }

    pub fn with_package(mut self, package: &str) -> Self {
        self.package = Some(package.to_string());
        self
    }

    pub fn add_import(&mut self, import: &str) {
        if !self.imports.contains(&import.to_string()) {
            self.imports.push(import.to_string());
        }
    }

    pub fn needs_timestamp_import(&self) -> bool {
        self.messages.iter().any(|m| m.uses_timestamp())
    }

    pub fn needs_any_import(&self) -> bool {
        self.messages.iter().any(|m| m.uses_any())
    }
}

impl Message {
    fn uses_timestamp(&self) -> bool {
        self.fields.iter().any(|f| matches!(&f.proto_type, ProtoType::Timestamp))
            || self.oneofs.iter().any(|o| o.fields.iter().any(|f| matches!(&f.proto_type, ProtoType::Timestamp)))
            || self.nested_messages.iter().any(|m| m.uses_timestamp())
    }

    fn uses_any(&self) -> bool {
        self.fields.iter().any(|f| matches!(&f.proto_type, ProtoType::Any))
            || self.oneofs.iter().any(|o| o.fields.iter().any(|f| matches!(&f.proto_type, ProtoType::Any)))
            || self.nested_messages.iter().any(|m| m.uses_any())
    }
}

impl fmt::Display for ProtoFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "syntax = \"{}\";", self.syntax)?;
        writeln!(f)?;

        // Write imports
        let mut imports = self.imports.clone();
        if self.needs_timestamp_import() {
            let ts_import = "google/protobuf/timestamp.proto".to_string();
            if !imports.contains(&ts_import) {
                imports.push(ts_import);
            }
        }
        if self.needs_any_import() {
            let any_import = "google/protobuf/any.proto".to_string();
            if !imports.contains(&any_import) {
                imports.push(any_import);
            }
        }

        for import in &imports {
            writeln!(f, "import \"{}\";", import)?;
        }
        if !imports.is_empty() {
            writeln!(f)?;
        }

        if let Some(pkg) = &self.package {
            writeln!(f, "package {};", pkg)?;
            writeln!(f)?;
        }

        for e in &self.enums {
            write!(f, "{}", e)?;
            writeln!(f)?;
        }

        for msg in &self.messages {
            write!(f, "{}", msg)?;
            writeln!(f)?;
        }

        Ok(())
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_message(f, self, 0)
    }
}

fn write_message(f: &mut fmt::Formatter<'_>, msg: &Message, indent: usize) -> fmt::Result {
    let prefix = "  ".repeat(indent);

    if let Some(comment) = &msg.comments {
        for line in comment.lines() {
            writeln!(f, "{}// {}", prefix, line)?;
        }
    }

    writeln!(f, "{}message {} {{", prefix, msg.name)?;

    // Nested enums
    for e in &msg.nested_enums {
        write_enum(f, e, indent + 1)?;
    }

    // Nested messages
    for nested in &msg.nested_messages {
        write_message(f, nested, indent + 1)?;
    }

    // Fields
    for field in &msg.fields {
        write_field(f, field, indent + 1)?;
    }

    // Oneofs
    for oneof in &msg.oneofs {
        write_oneof(f, oneof, indent + 1)?;
    }

    writeln!(f, "{}}}", prefix)?;

    Ok(())
}

fn write_field(f: &mut fmt::Formatter<'_>, field: &Field, indent: usize) -> fmt::Result {
    let prefix = "  ".repeat(indent);

    if let Some(comment) = &field.comments {
        for line in comment.lines() {
            writeln!(f, "{}// {}", prefix, line)?;
        }
    }

    let modifier = if field.repeated {
        "repeated "
    } else if field.optional {
        "optional "
    } else {
        ""
    };

    writeln!(
        f,
        "{}{}{} {} = {};",
        prefix, modifier, field.proto_type, field.name, field.number
    )?;

    Ok(())
}

fn write_oneof(f: &mut fmt::Formatter<'_>, oneof: &OneOf, indent: usize) -> fmt::Result {
    let prefix = "  ".repeat(indent);

    if let Some(comment) = &oneof.comments {
        for line in comment.lines() {
            writeln!(f, "{}// {}", prefix, line)?;
        }
    }

    writeln!(f, "{}oneof {} {{", prefix, oneof.name)?;

    for field in &oneof.fields {
        let inner_prefix = "  ".repeat(indent + 1);
        if let Some(comment) = &field.comments {
            for line in comment.lines() {
                writeln!(f, "{}// {}", inner_prefix, line)?;
            }
        }
        writeln!(
            f,
            "{}{} {} = {};",
            inner_prefix, field.proto_type, field.name, field.number
        )?;
    }

    writeln!(f, "{}}}", prefix)?;

    Ok(())
}

impl fmt::Display for Enum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_enum(f, self, 0)
    }
}

fn write_enum(f: &mut fmt::Formatter<'_>, e: &Enum, indent: usize) -> fmt::Result {
    let prefix = "  ".repeat(indent);

    if let Some(comment) = &e.comments {
        for line in comment.lines() {
            writeln!(f, "{}// {}", prefix, line)?;
        }
    }

    writeln!(f, "{}enum {} {{", prefix, e.name)?;

    for val in &e.values {
        writeln!(f, "{}  {} = {};", prefix, val.name, val.number)?;
    }

    writeln!(f, "{}}}", prefix)?;

    Ok(())
}
