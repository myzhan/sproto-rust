//! AST node types for parsed sproto schema.

/// A top-level item in a schema: either a type definition or a protocol.
#[derive(Debug, Clone)]
pub enum AstItem {
    Type(AstType),
    Protocol(AstProtocol),
}

/// A user-defined type (struct/message).
#[derive(Debug, Clone)]
pub struct AstType {
    pub name: String,
    pub members: Vec<AstMember>,
    pub line: usize,
}

/// A member of a type: either a field or a nested type.
#[derive(Debug, Clone)]
pub enum AstMember {
    Field(AstField),
    NestedType(AstType),
}

/// A field definition within a type.
#[derive(Debug, Clone)]
pub struct AstField {
    pub name: String,
    pub tag: u64,
    pub is_array: bool,
    pub type_name: String,
    /// For `*Type(key)` or `integer(precision)`: the parenthesized value.
    pub extra: Option<String>,
    pub line: usize,
}

/// A protocol (RPC) definition.
#[derive(Debug, Clone)]
pub struct AstProtocol {
    pub name: String,
    pub tag: u64,
    pub request: Option<AstProtoType>,
    pub response: Option<AstProtoType>,
    pub line: usize,
}

/// The type specification for a protocol request/response.
/// Can be either a reference to an existing type or an inline struct definition.
#[derive(Debug, Clone)]
pub enum AstProtoType {
    /// Reference to an existing type name.
    TypeName(String),
    /// Inline struct definition.
    InlineStruct(Vec<AstMember>),
    /// Explicitly nil (for "response nil").
    Nil,
}
