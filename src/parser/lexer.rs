/// Token types produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// `.` prefix for type definitions
    Dot,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `*` array prefix
    Star,
    /// `:`
    Colon,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// An identifier (field name, type name, keyword)
    Name(String),
    /// A numeric literal (tag number, decimal precision)
    Number(u64),
    /// End of input
    Eof,
}

/// A token with its source location.
#[derive(Debug, Clone)]
pub struct Located {
    pub token: Token,
    pub line: usize,
}

/// Tokenizer for sproto schema text.
pub struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
    line: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            input: input.as_bytes(),
            pos: 0,
            line: 1,
        }
    }

    pub fn current_line(&self) -> usize {
        self.line
    }

    fn peek_byte(&self) -> Option<u8> {
        if self.pos < self.input.len() {
            Some(self.input[self.pos])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<u8> {
        if self.pos < self.input.len() {
            let b = self.input[self.pos];
            self.pos += 1;
            if b == b'\n' {
                self.line += 1;
            }
            Some(b)
        } else {
            None
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek_byte() {
                Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n') => {
                    self.advance();
                }
                Some(b'#') => {
                    // Line comment: skip to end of line
                    while let Some(b) = self.advance() {
                        if b == b'\n' {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn read_name(&mut self) -> String {
        let start = self.pos;
        while let Some(b) = self.peek_byte() {
            if b.is_ascii_alphanumeric() || b == b'_' {
                self.advance();
            } else {
                break;
            }
        }
        String::from_utf8(self.input[start..self.pos].to_vec()).unwrap()
    }

    fn read_number(&mut self) -> u64 {
        let start = self.pos;
        while let Some(b) = self.peek_byte() {
            if b.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        let s = std::str::from_utf8(&self.input[start..self.pos]).unwrap();
        s.parse().unwrap()
    }

    /// Read the next token.
    pub fn next_token(&mut self) -> Located {
        self.skip_whitespace_and_comments();
        let line = self.line;

        match self.peek_byte() {
            None => Located {
                token: Token::Eof,
                line,
            },
            Some(b'.') => {
                self.advance();
                Located {
                    token: Token::Dot,
                    line,
                }
            }
            Some(b'{') => {
                self.advance();
                Located {
                    token: Token::LBrace,
                    line,
                }
            }
            Some(b'}') => {
                self.advance();
                Located {
                    token: Token::RBrace,
                    line,
                }
            }
            Some(b'*') => {
                self.advance();
                Located {
                    token: Token::Star,
                    line,
                }
            }
            Some(b':') => {
                self.advance();
                Located {
                    token: Token::Colon,
                    line,
                }
            }
            Some(b'(') => {
                self.advance();
                Located {
                    token: Token::LParen,
                    line,
                }
            }
            Some(b')') => {
                self.advance();
                Located {
                    token: Token::RParen,
                    line,
                }
            }
            Some(b) if b.is_ascii_alphabetic() || b == b'_' => {
                let name = self.read_name();
                Located {
                    token: Token::Name(name),
                    line,
                }
            }
            Some(b) if b.is_ascii_digit() => {
                let num = self.read_number();
                Located {
                    token: Token::Number(num),
                    line,
                }
            }
            Some(b) => {
                self.advance();
                Located {
                    token: Token::Name(format!("<unexpected '{}'>", b as char)),
                    line,
                }
            }
        }
    }

    /// Peek at the next token without consuming it.
    pub fn peek_token(&mut self) -> Located {
        let saved_pos = self.pos;
        let saved_line = self.line;
        let tok = self.next_token();
        self.pos = saved_pos;
        self.line = saved_line;
        tok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lex = Lexer::new(".Person { name 0 : string }");
        assert_eq!(lex.next_token().token, Token::Dot);
        assert_eq!(lex.next_token().token, Token::Name("Person".into()));
        assert_eq!(lex.next_token().token, Token::LBrace);
        assert_eq!(lex.next_token().token, Token::Name("name".into()));
        assert_eq!(lex.next_token().token, Token::Number(0));
        assert_eq!(lex.next_token().token, Token::Colon);
        assert_eq!(lex.next_token().token, Token::Name("string".into()));
        assert_eq!(lex.next_token().token, Token::RBrace);
        assert_eq!(lex.next_token().token, Token::Eof);
    }

    #[test]
    fn test_comments() {
        let mut lex = Lexer::new("# comment\n.Type {}");
        assert_eq!(lex.next_token().token, Token::Dot);
        assert_eq!(lex.next_token().token, Token::Name("Type".into()));
    }

    #[test]
    fn test_array_syntax() {
        let mut lex = Lexer::new("*integer");
        assert_eq!(lex.next_token().token, Token::Star);
        assert_eq!(lex.next_token().token, Token::Name("integer".into()));
    }

    #[test]
    fn test_line_tracking() {
        let mut lex = Lexer::new("a\nb\nc");
        let t1 = lex.next_token();
        assert_eq!(t1.line, 1);
        let t2 = lex.next_token();
        assert_eq!(t2.line, 2);
        let t3 = lex.next_token();
        assert_eq!(t3.line, 3);
    }
}
