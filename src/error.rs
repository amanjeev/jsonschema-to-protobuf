use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Failed to parse JSON Schema: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Unsupported JSON Schema feature: {0}")]
    UnsupportedFeature(String),

    #[error("Invalid reference: {0}")]
    InvalidRef(String),

    #[error("Circular reference detected: {0}")]
    CircularRef(String),

    #[error("Missing definition: {0}")]
    MissingDefinition(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ConversionError>;
