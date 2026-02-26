/// Errors from the sproto schema text parser.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("syntax error at line {line}: {message}")]
    Syntax { line: usize, message: String },

    #[error("duplicate tag {tag} in type '{type_name}'")]
    DuplicateTag { type_name: String, tag: u16 },

    #[error("duplicate field '{field_name}' in type '{type_name}'")]
    DuplicateField {
        type_name: String,
        field_name: String,
    },

    #[error("undefined type '{type_name}' referenced by '{referenced_by}'")]
    UndefinedType {
        type_name: String,
        referenced_by: String,
    },

    #[error("invalid map key: field '{field_name}' in type '{type_name}'")]
    InvalidMapKey {
        type_name: String,
        field_name: String,
    },

    #[error("redefined protocol tag {tag} at '{name}'")]
    DuplicateProtocolTag { tag: u16, name: String },

    #[error("redefined type '{name}'")]
    DuplicateType { name: String },
}

/// Errors from the binary encoder.
#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("type mismatch for field '{field}': expected {expected}, got {actual}")]
    TypeMismatch {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("unknown type '{0}'")]
    UnknownType(String),

    #[error("unknown protocol '{0}'")]
    UnknownProtocol(String),

    #[error("encode error: {0}")]
    Other(String),
}

/// Errors from the binary decoder.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("truncated data: need {need} bytes, have {have}")]
    Truncated { need: usize, have: usize },

    #[error("invalid data: {0}")]
    InvalidData(String),

    #[error("unknown type '{0}'")]
    UnknownType(String),

    #[error("unknown protocol '{0}'")]
    UnknownProtocol(String),

    #[error("invalid utf-8 string in field '{field}': {source}")]
    InvalidUtf8 {
        field: String,
        source: std::string::FromUtf8Error,
    },
}

/// Errors from the pack/unpack compression.
#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("invalid packed data: {0}")]
    InvalidData(String),
}

/// Errors from the RPC module.
#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error("unknown protocol '{0}'")]
    UnknownProtocol(String),

    #[error("unknown session {0}")]
    UnknownSession(u64),

    #[error("package type not found: '{0}'")]
    PackageTypeNotFound(String),

    #[error("encode error: {0}")]
    Encode(#[from] EncodeError),

    #[error("decode error: {0}")]
    Decode(#[from] DecodeError),

    #[error("pack error: {0}")]
    Pack(#[from] PackError),
}

/// Top-level error type that wraps all sub-errors.
#[derive(Debug, thiserror::Error)]
pub enum SprotoError {
    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error(transparent)]
    Encode(#[from] EncodeError),

    #[error(transparent)]
    Decode(#[from] DecodeError),

    #[error(transparent)]
    Pack(#[from] PackError),

    #[error(transparent)]
    Rpc(#[from] RpcError),
}

/// Result type alias for sproto operations.
pub type Result<T> = std::result::Result<T, SprotoError>;
