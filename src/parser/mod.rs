pub mod lexer;
pub mod ast;
pub mod grammar;
pub mod schema_builder;

use crate::error::ParseError;
use crate::types::Sproto;

/// Parse a sproto schema text string into a `Sproto` object.
///
/// This is the main entry point for the parser module.
pub fn parse(schema_text: &str) -> Result<Sproto, ParseError> {
    let ast_items = grammar::parse_schema(schema_text)?;
    schema_builder::build_schema(ast_items)
}
