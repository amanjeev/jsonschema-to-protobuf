use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jsonschema_to_protobuf::{convert, Options};

const SIMPLE_SCHEMA: &str = r#"{
    "title": "Simple",
    "type": "object",
    "properties": {
        "name": { "type": "string" },
        "age": { "type": "integer" }
    }
}"#;

const MEDIUM_SCHEMA: &str = r#"{
    "title": "User",
    "type": "object",
    "properties": {
        "id": { "type": "integer" },
        "username": { "type": "string" },
        "email": { "type": "string" },
        "createdAt": { "type": "string", "format": "date-time" },
        "roles": {
            "type": "array",
            "items": { "type": "string", "enum": ["admin", "user", "guest"] }
        },
        "profile": {
            "type": "object",
            "title": "Profile",
            "properties": {
                "displayName": { "type": "string" },
                "bio": { "type": "string" }
            }
        },
        "settings": {
            "type": "object",
            "additionalProperties": { "type": "string" }
        }
    },
    "required": ["id", "username"]
}"#;

const COMPLEX_SCHEMA: &str = r##"{
    "title": "ApiResponse",
    "type": "object",
    "definitions": {
        "Error": {
            "type": "object",
            "properties": {
                "code": { "type": "integer" },
                "message": { "type": "string" },
                "details": { "type": "array", "items": { "type": "string" } }
            }
        },
        "Pagination": {
            "type": "object",
            "properties": {
                "page": { "type": "integer" },
                "perPage": { "type": "integer" },
                "total": { "type": "integer" }
            }
        },
        "User": {
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "name": { "type": "string" },
                "email": { "type": "string" }
            }
        },
        "Post": {
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "title": { "type": "string" },
                "content": { "type": "string" },
                "author": { "$ref": "#/definitions/User" }
            }
        }
    },
    "properties": {
        "success": { "type": "boolean" },
        "data": {
            "oneOf": [
                { "$ref": "#/definitions/User" },
                { "$ref": "#/definitions/Post" },
                { "type": "array", "items": { "$ref": "#/definitions/User" } }
            ]
        },
        "error": { "$ref": "#/definitions/Error" },
        "pagination": { "$ref": "#/definitions/Pagination" },
        "timestamp": { "type": "string", "format": "date-time" }
    }
}"##;

fn bench_simple(c: &mut Criterion) {
    c.bench_function("convert_simple", |b| {
        b.iter(|| convert(black_box(SIMPLE_SCHEMA), Options::default()))
    });
}

fn bench_medium(c: &mut Criterion) {
    c.bench_function("convert_medium", |b| {
        b.iter(|| convert(black_box(MEDIUM_SCHEMA), Options::default()))
    });
}

fn bench_complex(c: &mut Criterion) {
    c.bench_function("convert_complex", |b| {
        b.iter(|| convert(black_box(COMPLEX_SCHEMA), Options::default()))
    });
}

fn bench_with_package(c: &mut Criterion) {
    let options = Options {
        package: Some("myapi.v1".to_string()),
    };
    c.bench_function("convert_with_package", |b| {
        b.iter(|| convert(black_box(MEDIUM_SCHEMA), options.clone()))
    });
}

fn bench_output_generation(c: &mut Criterion) {
    let proto = convert(COMPLEX_SCHEMA, Options::default()).unwrap();
    c.bench_function("proto_to_string", |b| {
        b.iter(|| black_box(&proto).to_string())
    });
}

criterion_group!(
    benches,
    bench_simple,
    bench_medium,
    bench_complex,
    bench_with_package,
    bench_output_generation
);
criterion_main!(benches);
