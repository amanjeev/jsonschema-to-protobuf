use jsonschema_to_protobuf::{convert, Options};

fn main() {
    let schema = r#"{
        "title": "Person",
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "The person's full name"
            },
            "age": {
                "type": "integer",
                "description": "Age in years"
            },
            "email": {
                "type": "string",
                "format": "email"
            }
        },
        "required": ["name"]
    }"#;

    let options = Options {
        package: Some("example.v1".to_string()),
    };

    match convert(schema, options) {
        Ok(proto) => {
            println!("{}", proto);
        }
        Err(e) => {
            eprintln!("Conversion error: {}", e);
        }
    }
}
