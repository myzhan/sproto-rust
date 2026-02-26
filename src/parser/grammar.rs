use crate::error::ParseError;
use super::ast::*;
use super::lexer::{Lexer, Token};

/// Parse a sproto schema text into an AST.
pub fn parse_schema(input: &str) -> Result<Vec<AstItem>, ParseError> {
    let mut lexer = Lexer::new(input);
    let mut items = Vec::new();

    loop {
        let tok = lexer.peek_token();
        match &tok.token {
            Token::Eof => break,
            Token::Dot => {
                items.push(AstItem::Type(parse_type_def(&mut lexer)?));
            }
            Token::Name(_) => {
                items.push(AstItem::Protocol(parse_protocol_def(&mut lexer)?));
            }
            _ => {
                return Err(ParseError::Syntax {
                    line: tok.line,
                    message: format!("expected type definition (.) or protocol name, found {:?}", tok.token),
                });
            }
        }
    }

    Ok(items)
}

fn parse_type_def(lexer: &mut Lexer) -> Result<AstType, ParseError> {
    // Consume '.'
    let dot = lexer.next_token();
    assert_eq!(dot.token, Token::Dot);
    let line = dot.line;

    // Name
    let name = expect_name(lexer)?;

    // '{'
    expect_token(lexer, Token::LBrace)?;

    // Members (fields or nested types)
    let members = parse_members(lexer)?;

    // '}'
    expect_token(lexer, Token::RBrace)?;

    Ok(AstType {
        name,
        members,
        line,
    })
}

fn parse_members(lexer: &mut Lexer) -> Result<Vec<AstMember>, ParseError> {
    let mut members = Vec::new();

    loop {
        let tok = lexer.peek_token();
        match &tok.token {
            Token::RBrace | Token::Eof => break,
            Token::Dot => {
                let nested = parse_type_def(lexer)?;
                members.push(AstMember::NestedType(nested));
            }
            Token::Name(_) => {
                let field = parse_field(lexer)?;
                members.push(AstMember::Field(field));
            }
            _ => {
                return Err(ParseError::Syntax {
                    line: tok.line,
                    message: format!("expected field name, nested type, or '}}', found {:?}", tok.token),
                });
            }
        }
    }

    Ok(members)
}

fn parse_field(lexer: &mut Lexer) -> Result<AstField, ParseError> {
    let line = lexer.current_line();

    // field_name
    let name = expect_name(lexer)?;

    // tag (number)
    let tag = expect_number(lexer)?;

    // ':'
    expect_token(lexer, Token::Colon)?;

    // Optional '*' for array
    let is_array = if lexer.peek_token().token == Token::Star {
        lexer.next_token();
        true
    } else {
        false
    };

    // type_name (simple name; nested type resolution is done by schema_builder)
    let type_name = expect_name(lexer)?;

    // Optional '(' key_or_decimal ')'
    let extra = if lexer.peek_token().token == Token::LParen {
        lexer.next_token(); // consume '('
        let tok = lexer.peek_token();
        let extra_val = match &tok.token {
            Token::RParen => {
                // Empty parens: *Type()
                String::new()
            }
            Token::Name(_) => {
                expect_name(lexer)?
            }
            Token::Number(n) => {
                let n = *n;
                lexer.next_token();
                n.to_string()
            }
            _ => {
                return Err(ParseError::Syntax {
                    line: tok.line,
                    message: format!("expected name, number, or ')' in parentheses, found {:?}", tok.token),
                });
            }
        };
        expect_token(lexer, Token::RParen)?;
        Some(extra_val)
    } else {
        None
    };

    Ok(AstField {
        name,
        tag,
        is_array,
        type_name,
        extra,
        line,
    })
}

fn parse_protocol_def(lexer: &mut Lexer) -> Result<AstProtocol, ParseError> {
    let line = lexer.current_line();

    // protocol_name
    let name = expect_name(lexer)?;

    // tag (number)
    let tag = expect_number(lexer)?;

    // '{'
    expect_token(lexer, Token::LBrace)?;

    let mut request = None;
    let mut response = None;

    loop {
        let tok = lexer.peek_token();
        match &tok.token {
            Token::RBrace | Token::Eof => break,
            Token::Name(n) if n == "request" => {
                lexer.next_token();
                request = Some(parse_proto_type(lexer)?);
            }
            Token::Name(n) if n == "response" => {
                lexer.next_token();
                response = Some(parse_proto_type(lexer)?);
            }
            _ => {
                return Err(ParseError::Syntax {
                    line: tok.line,
                    message: format!("expected 'request', 'response', or '}}', found {:?}", tok.token),
                });
            }
        }
    }

    // '}'
    expect_token(lexer, Token::RBrace)?;

    Ok(AstProtocol {
        name,
        tag,
        request,
        response,
        line,
    })
}

fn parse_proto_type(lexer: &mut Lexer) -> Result<AstProtoType, ParseError> {
    let tok = lexer.peek_token();
    match &tok.token {
        Token::LBrace => {
            lexer.next_token();
            let members = parse_members(lexer)?;
            expect_token(lexer, Token::RBrace)?;
            Ok(AstProtoType::InlineStruct(members))
        }
        Token::Name(n) if n == "nil" => {
            lexer.next_token();
            Ok(AstProtoType::Nil)
        }
        Token::Name(_) => {
            let name = expect_name(lexer)?;
            Ok(AstProtoType::TypeName(name))
        }
        _ => Err(ParseError::Syntax {
            line: tok.line,
            message: format!("expected type name or inline struct, found {:?}", tok.token),
        }),
    }
}

// Helper functions

fn expect_name(lexer: &mut Lexer) -> Result<String, ParseError> {
    let tok = lexer.next_token();
    match tok.token {
        Token::Name(n) => Ok(n),
        _ => Err(ParseError::Syntax {
            line: tok.line,
            message: format!("expected name, found {:?}", tok.token),
        }),
    }
}

fn expect_number(lexer: &mut Lexer) -> Result<u64, ParseError> {
    let tok = lexer.next_token();
    match tok.token {
        Token::Number(n) => Ok(n),
        _ => Err(ParseError::Syntax {
            line: tok.line,
            message: format!("expected number, found {:?}", tok.token),
        }),
    }
}

fn expect_token(lexer: &mut Lexer, expected: Token) -> Result<(), ParseError> {
    let tok = lexer.next_token();
    if tok.token == expected {
        Ok(())
    } else {
        Err(ParseError::Syntax {
            line: tok.line,
            message: format!("expected {:?}, found {:?}", expected, tok.token),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_type() {
        let schema = ".Person { name 0 : string  id 1 : integer }";
        let items = parse_schema(schema).unwrap();
        assert_eq!(items.len(), 1);
        match &items[0] {
            AstItem::Type(t) => {
                assert_eq!(t.name, "Person");
                assert_eq!(t.members.len(), 2);
            }
            _ => panic!("expected type"),
        }
    }

    #[test]
    fn test_parse_array_field() {
        let schema = ".Data { numbers 0 : *integer }";
        let items = parse_schema(schema).unwrap();
        match &items[0] {
            AstItem::Type(t) => {
                match &t.members[0] {
                    AstMember::Field(f) => {
                        assert!(f.is_array);
                        assert_eq!(f.type_name, "integer");
                    }
                    _ => panic!("expected field"),
                }
            }
            _ => panic!("expected type"),
        }
    }

    #[test]
    fn test_parse_protocol() {
        let schema = "foobar 1 { request Person  response { ok 0 : boolean } }";
        let items = parse_schema(schema).unwrap();
        match &items[0] {
            AstItem::Protocol(p) => {
                assert_eq!(p.name, "foobar");
                assert_eq!(p.tag, 1);
                assert!(p.request.is_some());
                assert!(p.response.is_some());
            }
            _ => panic!("expected protocol"),
        }
    }

    #[test]
    fn test_parse_nested_type() {
        let schema = r#"
        .Person {
            name 0 : string
            .PhoneNumber {
                number 0 : string
                type 1 : integer
            }
            phone 1 : *PhoneNumber
        }
        "#;
        let items = parse_schema(schema).unwrap();
        match &items[0] {
            AstItem::Type(t) => {
                assert_eq!(t.name, "Person");
                assert_eq!(t.members.len(), 3); // name, PhoneNumber, phone
            }
            _ => panic!("expected type"),
        }
    }

    #[test]
    fn test_parse_map_syntax() {
        let schema = ".AddressBook { person 0 : *Person(id) }";
        let items = parse_schema(schema).unwrap();
        match &items[0] {
            AstItem::Type(t) => {
                match &t.members[0] {
                    AstMember::Field(f) => {
                        assert!(f.is_array);
                        assert_eq!(f.extra.as_deref(), Some("id"));
                    }
                    _ => panic!("expected field"),
                }
            }
            _ => panic!("expected type"),
        }
    }

    #[test]
    fn test_parse_response_nil() {
        let schema = "bar 3 { response nil }";
        let items = parse_schema(schema).unwrap();
        match &items[0] {
            AstItem::Protocol(p) => {
                assert!(matches!(&p.response, Some(AstProtoType::Nil)));
            }
            _ => panic!("expected protocol"),
        }
    }
}
