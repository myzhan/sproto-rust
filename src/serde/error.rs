//! Serde error types for sproto serialization/deserialization.

use std::fmt::Display;

use crate::error::{DecodeError, EncodeError};

/// Error type for serde serialization/deserialization operations.
#[derive(Debug)]
pub enum SerdeError {
    /// Type mismatch between expected and actual value types.
    TypeMismatch {
        field: String,
        expected: String,
        actual: String,
    },
    /// A required field is missing during deserialization.
    MissingField(String),
    /// The type is not supported for sproto serialization.
    UnsupportedType(String),
    /// Error during encoding.
    Encode(EncodeError),
    /// Error during decoding.
    Decode(DecodeError),
    /// Custom error message.
    Custom(String),
}

impl Display for SerdeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerdeError::TypeMismatch {
                field,
                expected,
                actual,
            } => write!(
                f,
                "type mismatch for field '{}': expected {}, got {}",
                field, expected, actual
            ),
            SerdeError::MissingField(name) => write!(f, "missing required field '{}'", name),
            SerdeError::UnsupportedType(ty) => write!(f, "unsupported type: {}", ty),
            SerdeError::Encode(e) => write!(f, "encode error: {}", e),
            SerdeError::Decode(e) => write!(f, "decode error: {}", e),
            SerdeError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for SerdeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SerdeError::Encode(e) => Some(e),
            SerdeError::Decode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<EncodeError> for SerdeError {
    fn from(e: EncodeError) -> Self {
        SerdeError::Encode(e)
    }
}

impl From<DecodeError> for SerdeError {
    fn from(e: DecodeError) -> Self {
        SerdeError::Decode(e)
    }
}

impl serde::ser::Error for SerdeError {
    fn custom<T: Display>(msg: T) -> Self {
        SerdeError::Custom(msg.to_string())
    }
}

impl serde::de::Error for SerdeError {
    fn custom<T: Display>(msg: T) -> Self {
        SerdeError::Custom(msg.to_string())
    }
}
