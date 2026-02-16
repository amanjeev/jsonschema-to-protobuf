use jsonschema_to_protobuf::{convert, Options};

#[test]
fn test_basic_types() {
    let schema = r#"{
        "title": "BasicTypes",
        "type": "object",
        "properties": {
            "stringField": { "type": "string" },
            "intField": { "type": "integer" },
            "numField": { "type": "number" },
            "boolField": { "type": "boolean" }
        }
    }"#;

    let result = convert(schema, Options::default()).unwrap();
    let output = result.to_string();

    assert!(output.contains("message BasicTypes"));
    assert!(output.contains("string string_field"));
    assert!(output.contains("int64 int_field"));
    assert!(output.contains("double num_field"));
    assert!(output.contains("bool bool_field"));
}

#[test]
fn test_array_type() {
    let schema = r#"{
        "title": "WithArray",
        "type": "object",
        "properties": {
            "tags": {
                "type": "array",
                "items": { "type": "string" }
            }
        }
    }"#;

    let result = convert(schema, Options::default()).unwrap();
    let output = result.to_string();

    assert!(output.contains("repeated string tags"));
}

#[test]
fn test_nested_object() {
    let schema = r#"{
        "title": "Parent",
        "type": "object",
        "properties": {
            "child": {
                "type": "object",
                "title": "Child",
                "properties": {
                    "name": { "type": "string" }
                }
            }
        }
    }"#;

    let result = convert(schema, Options::default()).unwrap();
    let output = result.to_string();

    assert!(output.contains("message Parent"));
    assert!(output.contains("message Child"));
    assert!(output.contains("Child child"));
}

#[test]
fn test_enum() {
    let schema = r#"{
        "title": "WithStatus",
        "type": "object",
        "properties": {
            "status": {
                "type": "string",
                "title": "Status",
                "enum": ["pending", "active", "completed"]
            }
        }
    }"#;

    let result = convert(schema, Options::default()).unwrap();
    let output = result.to_string();

    assert!(output.contains("enum Status"));
    assert!(output.contains("STATUS_UNSPECIFIED = 0"));
    assert!(output.contains("STATUS_PENDING = 1"));
    assert!(output.contains("STATUS_ACTIVE = 2"));
    assert!(output.contains("STATUS_COMPLETED = 3"));
}

#[test]
fn test_ref_to_definitions() {
    let schema = r##"{
        "title": "Root",
        "type": "object",
        "definitions": {
            "Address": {
                "type": "object",
                "properties": {
                    "street": { "type": "string" },
                    "city": { "type": "string" }
                }
            }
        },
        "properties": {
            "homeAddress": { "$ref": "#/definitions/Address" },
            "workAddress": { "$ref": "#/definitions/Address" }
        }
    }"##;

    let result = convert(schema, Options::default()).unwrap();
    let output = result.to_string();

    assert!(output.contains("message Address"));
    assert!(output.contains("Address home_address"));
    assert!(output.contains("Address work_address"));
}

#[test]
fn test_one_of() {
    let schema = r##"{
        "title": "Response",
        "type": "object",
        "definitions": {
            "Success": {
                "type": "object",
                "properties": { "data": { "type": "string" } }
            },
            "Failure": {
                "type": "object",
                "properties": { "error": { "type": "string" } }
            }
        },
        "properties": {
            "result": {
                "oneOf": [
                    { "$ref": "#/definitions/Success" },
                    { "$ref": "#/definitions/Failure" }
                ]
            }
        }
    }"##;

    let result = convert(schema, Options::default()).unwrap();
    let output = result.to_string();

    assert!(output.contains("message Success"));
    assert!(output.contains("message Failure"));
    assert!(output.contains("oneof value"));
    assert!(output.contains("Success success"));
    assert!(output.contains("Failure failure"));
}

#[test]
fn test_all_of_merge() {
    let schema = r#"{
        "title": "Combined",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "id": { "type": "integer" }
                }
            },
            {
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                }
            }
        ]
    }"#;

    let result = convert(schema, Options::default()).unwrap();
    let output = result.to_string();

    assert!(output.contains("message Combined"));
    assert!(output.contains("int64 id"));
    assert!(output.contains("string name"));
}

#[test]
fn test_map_type() {
    let schema = r#"{
        "title": "WithMap",
        "type": "object",
        "properties": {
            "metadata": {
                "type": "object",
                "additionalProperties": { "type": "string" }
            }
        }
    }"#;

    let result = convert(schema, Options::default()).unwrap();
    let output = result.to_string();

    assert!(output.contains("map<string, string> metadata"));
}

#[test]
fn test_datetime_format() {
    let schema = r#"{
        "title": "Event",
        "type": "object",
        "properties": {
            "createdAt": {
                "type": "string",
                "format": "date-time"
            }
        }
    }"#;

    let result = convert(schema, Options::default()).unwrap();
    let output = result.to_string();

    assert!(output.contains("google.protobuf.Timestamp created_at"));
    assert!(output.contains("import \"google/protobuf/timestamp.proto\""));
}

#[test]
fn test_optional_and_required() {
    let schema = r#"{
        "title": "User",
        "type": "object",
        "properties": {
            "id": { "type": "integer" },
            "email": { "type": "string" }
        },
        "required": ["id"]
    }"#;

    let result = convert(schema, Options::default()).unwrap();
    let output = result.to_string();

    // id is required, so no optional
    assert!(output.contains("int64 id ="));
    // email is not required, so optional
    assert!(output.contains("optional string email"));
}

#[test]
fn test_package_option() {
    let schema = r#"{
        "title": "Test",
        "type": "object",
        "properties": { "x": { "type": "integer" } }
    }"#;

    let options = Options {
        package: Some("mycompany.api.v1".to_string()),
    };

    let result = convert(schema, options).unwrap();
    let output = result.to_string();

    assert!(output.contains("package mycompany.api.v1;"));
}

#[test]
fn test_nullable_type() {
    let schema = r#"{
        "title": "Nullable",
        "type": "object",
        "properties": {
            "value": { "type": ["string", "null"] }
        }
    }"#;

    let result = convert(schema, Options::default()).unwrap();
    let output = result.to_string();

    assert!(output.contains("optional string value"));
}
