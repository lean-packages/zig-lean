use anyhow::{Result, anyhow, bail};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Whitespace,
    Comment,
    Dot,
    Comma,
    Colon,
    Semicolon,
    Equal,
    FatArrow,
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Identifier(String),
    DotIdentifier(String),
    StringLiteral(String),
    CharLiteral(char),
    NumberLiteral(String),
    BoolLiteral(bool),
    NullLiteral,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: (usize, usize),
    pub line: usize,
    pub column: usize,
}

pub struct Lexer<'a> {
    input: &'a str,
    chars: Vec<(usize, char)>,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let chars: Vec<(usize, char)> = input.char_indices().collect();
        Self {
            input,
            chars,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).map(|&(_, c)| c)
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).map(|&(_, c)| c)
    }

    fn current_byte_offset(&self) -> usize {
        self.chars
            .get(self.pos)
            .map(|&(idx, _)| idx)
            .unwrap_or(self.input.len())
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(&(_, c)) = self.chars.get(self.pos) {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            Some(c)
        } else {
            None
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        while let Some(c) = self.peek() {
            let start_byte = self.current_byte_offset();
            let start_line = self.line;
            let start_col = self.col;

            if c.is_whitespace() {
                while let Some(w) = self.peek() {
                    if w.is_whitespace() {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let end_byte = self.current_byte_offset();
                tokens.push(Token {
                    kind: TokenKind::Whitespace,
                    span: (start_byte, end_byte),
                    line: start_line,
                    column: start_col,
                });
                continue;
            }

            // Line comment or multiline string
            if c == '/' && self.peek_next() == Some('/') {
                self.advance(); // /
                self.advance(); // /
                while let Some(nc) = self.peek() {
                    if nc == '\n' {
                        break;
                    }
                    self.advance();
                }
                let end_byte = self.current_byte_offset();
                tokens.push(Token {
                    kind: TokenKind::Comment,
                    span: (start_byte, end_byte),
                    line: start_line,
                    column: start_col,
                });
                continue;
            }

            // Multiline string literal (starts with \\)
            if c == '\\' && self.peek_next() == Some('\\') {
                let mut content = String::new();
                let mut first = true;

                while self.peek() == Some('\\') && self.peek_next() == Some('\\') {
                    if !first {
                        content.push('\n');
                    }
                    first = false;
                    self.advance(); // \
                    self.advance(); // \

                    while let Some(nc) = self.peek() {
                        if nc == '\n' {
                            self.advance();
                            break;
                        } else if nc == '\r' && self.peek_next() == Some('\n') {
                            self.advance();
                            self.advance();
                            break;
                        } else {
                            content.push(nc);
                            self.advance();
                        }
                    }

                    // Skip leading whitespace to next line if next line also starts with \\
                    let save_pos = self.pos;
                    let save_line = self.line;
                    let save_col = self.col;
                    while let Some(ws) = self.peek() {
                        if ws == ' ' || ws == '\t' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if self.peek() != Some('\\') || self.peek_next() != Some('\\') {
                        self.pos = save_pos;
                        self.line = save_line;
                        self.col = save_col;
                        break;
                    }
                }

                let end_byte = self.current_byte_offset();
                tokens.push(Token {
                    kind: TokenKind::StringLiteral(content),
                    span: (start_byte, end_byte),
                    line: start_line,
                    column: start_col,
                });
                continue;
            }

            // String literal "..."
            if c == '"' {
                self.advance();
                let mut string_val = String::new();
                let mut closed = false;

                while let Some(sc) = self.peek() {
                    if sc == '"' {
                        self.advance();
                        closed = true;
                        break;
                    } else if sc == '\\' {
                        self.advance();
                        if let Some(esc) = self.advance() {
                            match esc {
                                'n' => string_val.push('\n'),
                                'r' => string_val.push('\r'),
                                't' => string_val.push('\t'),
                                '\\' => string_val.push('\\'),
                                '\'' => string_val.push('\''),
                                '"' => string_val.push('"'),
                                'x' => {
                                    let h1 = self.advance().ok_or_else(|| {
                                        anyhow!("Unexpected EOF in hex escape at {start_line}:{start_col}")
                                    })?;
                                    let h2 = self.advance().ok_or_else(|| {
                                        anyhow!("Unexpected EOF in hex escape at {start_line}:{start_col}")
                                    })?;
                                    let hex_str: String = [h1, h2].iter().collect();
                                    let byte = u8::from_str_radix(&hex_str, 16).map_err(|_| {
                                        anyhow!("Invalid hex escape '\\x{hex_str}' at {start_line}:{start_col}")
                                    })?;
                                    string_val.push(byte as char);
                                }
                                'u' => {
                                    if self.advance() != Some('{') {
                                        bail!(
                                            "Expected '{{' in unicode escape at {start_line}:{start_col}"
                                        );
                                    }
                                    let mut hex = String::new();
                                    while let Some(uc) = self.peek() {
                                        if uc == '}' {
                                            self.advance();
                                            break;
                                        }
                                        hex.push(uc);
                                        self.advance();
                                    }
                                    let codepoint = u32::from_str_radix(&hex, 16).map_err(|_| {
                                        anyhow!("Invalid unicode escape '\\u{{{hex}}}' at {start_line}:{start_col}")
                                    })?;
                                    let ch = char::from_u32(codepoint).ok_or_else(|| {
                                        anyhow!("Invalid codepoint in '\\u{{{hex}}}' at {start_line}:{start_col}")
                                    })?;
                                    string_val.push(ch);
                                }
                                other => {
                                    string_val.push('\\');
                                    string_val.push(other);
                                }
                            }
                        } else {
                            bail!(
                                "Unexpected EOF in string literal escape at {start_line}:{start_col}"
                            );
                        }
                    } else if sc == '\n' {
                        bail!("Unescaped newline in string literal at {start_line}:{start_col}");
                    } else {
                        string_val.push(sc);
                        self.advance();
                    }
                }

                if !closed {
                    bail!("Unterminated string literal starting at {start_line}:{start_col}");
                }

                let end_byte = self.current_byte_offset();
                tokens.push(Token {
                    kind: TokenKind::StringLiteral(string_val),
                    span: (start_byte, end_byte),
                    line: start_line,
                    column: start_col,
                });
                continue;
            }

            // Character literal '...'
            if c == '\'' {
                self.advance();
                let mut ch_val = '\0';
                if let Some(mut ch) = self.advance() {
                    if ch == '\\' {
                        let esc = self.advance().ok_or_else(|| {
                            anyhow!(
                                "Unexpected EOF in char literal escape at {start_line}:{start_col}"
                            )
                        })?;
                        ch = match esc {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '\\' => '\\',
                            '\'' => '\'',
                            '"' => '"',
                            other => other,
                        };
                    }
                    ch_val = ch;
                }
                if self.peek() == Some('\'') {
                    self.advance();
                } else {
                    bail!(
                        "Unterminated or multi-character char literal at {start_line}:{start_col}"
                    );
                }
                let end_byte = self.current_byte_offset();
                tokens.push(Token {
                    kind: TokenKind::CharLiteral(ch_val),
                    span: (start_byte, end_byte),
                    line: start_line,
                    column: start_col,
                });
                continue;
            }

            // Dot and DotIdentifier (.foo or .@"foo")
            if c == '.' {
                self.advance();
                // Check if followed by identifier or @"..."
                if let Some(nc) = self.peek() {
                    if nc == '@' && self.peek_next() == Some('"') {
                        self.advance(); // @
                        self.advance(); // "
                        let mut ident = String::new();
                        let mut closed = false;
                        while let Some(ic) = self.peek() {
                            if ic == '"' {
                                self.advance();
                                closed = true;
                                break;
                            } else {
                                ident.push(ic);
                                self.advance();
                            }
                        }
                        if !closed {
                            bail!("Unterminated @\" identifier at {start_line}:{start_col}");
                        }
                        let end_byte = self.current_byte_offset();
                        tokens.push(Token {
                            kind: TokenKind::DotIdentifier(ident),
                            span: (start_byte, end_byte),
                            line: start_line,
                            column: start_col,
                        });
                        continue;
                    } else if nc.is_ascii_alphabetic() || nc == '_' {
                        let mut ident = String::new();
                        while let Some(ic) = self.peek() {
                            if ic.is_ascii_alphanumeric() || ic == '_' {
                                ident.push(ic);
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        let end_byte = self.current_byte_offset();
                        tokens.push(Token {
                            kind: TokenKind::DotIdentifier(ident),
                            span: (start_byte, end_byte),
                            line: start_line,
                            column: start_col,
                        });
                        continue;
                    }
                }

                let end_byte = self.current_byte_offset();
                tokens.push(Token {
                    kind: TokenKind::Dot,
                    span: (start_byte, end_byte),
                    line: start_line,
                    column: start_col,
                });
                continue;
            }

            // Identifiers with @"..."
            if c == '@' && self.peek_next() == Some('"') {
                self.advance(); // @
                self.advance(); // "
                let mut ident = String::new();
                let mut closed = false;
                while let Some(ic) = self.peek() {
                    if ic == '"' {
                        self.advance();
                        closed = true;
                        break;
                    } else {
                        ident.push(ic);
                        self.advance();
                    }
                }
                if !closed {
                    bail!("Unterminated @\" identifier at {start_line}:{start_col}");
                }
                let end_byte = self.current_byte_offset();
                tokens.push(Token {
                    kind: TokenKind::Identifier(ident),
                    span: (start_byte, end_byte),
                    line: start_line,
                    column: start_col,
                });
                continue;
            }

            // Single and multi-char punctuation
            match c {
                ',' => {
                    self.advance();
                    let end_byte = self.current_byte_offset();
                    tokens.push(Token {
                        kind: TokenKind::Comma,
                        span: (start_byte, end_byte),
                        line: start_line,
                        column: start_col,
                    });
                    continue;
                }
                ':' => {
                    self.advance();
                    let end_byte = self.current_byte_offset();
                    tokens.push(Token {
                        kind: TokenKind::Colon,
                        span: (start_byte, end_byte),
                        line: start_line,
                        column: start_col,
                    });
                    continue;
                }
                ';' => {
                    self.advance();
                    let end_byte = self.current_byte_offset();
                    tokens.push(Token {
                        kind: TokenKind::Semicolon,
                        span: (start_byte, end_byte),
                        line: start_line,
                        column: start_col,
                    });
                    continue;
                }
                '=' => {
                    self.advance();
                    if self.peek() == Some('>') {
                        self.advance();
                        let end_byte = self.current_byte_offset();
                        tokens.push(Token {
                            kind: TokenKind::FatArrow,
                            span: (start_byte, end_byte),
                            line: start_line,
                            column: start_col,
                        });
                    } else {
                        let end_byte = self.current_byte_offset();
                        tokens.push(Token {
                            kind: TokenKind::Equal,
                            span: (start_byte, end_byte),
                            line: start_line,
                            column: start_col,
                        });
                    }
                    continue;
                }
                '{' => {
                    self.advance();
                    let end_byte = self.current_byte_offset();
                    tokens.push(Token {
                        kind: TokenKind::LBrace,
                        span: (start_byte, end_byte),
                        line: start_line,
                        column: start_col,
                    });
                    continue;
                }
                '}' => {
                    self.advance();
                    let end_byte = self.current_byte_offset();
                    tokens.push(Token {
                        kind: TokenKind::RBrace,
                        span: (start_byte, end_byte),
                        line: start_line,
                        column: start_col,
                    });
                    continue;
                }
                '(' => {
                    self.advance();
                    let end_byte = self.current_byte_offset();
                    tokens.push(Token {
                        kind: TokenKind::LParen,
                        span: (start_byte, end_byte),
                        line: start_line,
                        column: start_col,
                    });
                    continue;
                }
                ')' => {
                    self.advance();
                    let end_byte = self.current_byte_offset();
                    tokens.push(Token {
                        kind: TokenKind::RParen,
                        span: (start_byte, end_byte),
                        line: start_line,
                        column: start_col,
                    });
                    continue;
                }
                '[' => {
                    self.advance();
                    let end_byte = self.current_byte_offset();
                    tokens.push(Token {
                        kind: TokenKind::LBracket,
                        span: (start_byte, end_byte),
                        line: start_line,
                        column: start_col,
                    });
                    continue;
                }
                ']' => {
                    self.advance();
                    let end_byte = self.current_byte_offset();
                    tokens.push(Token {
                        kind: TokenKind::RBracket,
                        span: (start_byte, end_byte),
                        line: start_line,
                        column: start_col,
                    });
                    continue;
                }
                _ => {}
            }

            // Identifiers / Keywords
            if c.is_ascii_alphabetic() || c == '_' {
                let mut ident = String::new();
                while let Some(ic) = self.peek() {
                    if ic.is_ascii_alphanumeric() || ic == '_' {
                        ident.push(ic);
                        self.advance();
                    } else {
                        break;
                    }
                }
                let end_byte = self.current_byte_offset();
                let kind = match ident.as_str() {
                    "true" => TokenKind::BoolLiteral(true),
                    "false" => TokenKind::BoolLiteral(false),
                    "null" => TokenKind::NullLiteral,
                    _ => TokenKind::Identifier(ident),
                };
                tokens.push(Token {
                    kind,
                    span: (start_byte, end_byte),
                    line: start_line,
                    column: start_col,
                });
                continue;
            }

            // Number literals
            if c.is_ascii_digit()
                || (c == '-' && self.peek_next().is_some_and(|n| n.is_ascii_digit()))
            {
                let mut num = String::new();
                if c == '-' {
                    num.push(c);
                    self.advance();
                }

                if self.peek() == Some('0') {
                    num.push('0');
                    self.advance();
                    if let Some(nc) = self.peek()
                        && (nc == 'x'
                            || nc == 'X'
                            || nc == 'o'
                            || nc == 'O'
                            || nc == 'b'
                            || nc == 'B')
                    {
                        num.push(nc);
                        self.advance();
                    }
                }

                while let Some(nc) = self.peek() {
                    if nc.is_ascii_hexdigit()
                        || nc == '_'
                        || nc == '.'
                        || nc == 'e'
                        || nc == 'E'
                        || nc == 'p'
                        || nc == 'P'
                        || nc == '+'
                        || nc == '-'
                    {
                        // Avoid treating .paths as part of number
                        if nc == '.'
                            && self.peek_next().is_some_and(|next| {
                                next.is_ascii_alphabetic() || next == '@' || next == '{'
                            })
                        {
                            break;
                        }
                        num.push(nc);
                        self.advance();
                    } else {
                        break;
                    }
                }

                let end_byte = self.current_byte_offset();
                tokens.push(Token {
                    kind: TokenKind::NumberLiteral(num),
                    span: (start_byte, end_byte),
                    line: start_line,
                    column: start_col,
                });
                continue;
            }

            // Unknown character
            self.advance();
            bail!("Unexpected character '{c}' at {start_line}:{start_col}");
        }

        Ok(tokens)
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Whitespace => write!(f, "whitespace"),
            TokenKind::Comment => write!(f, "comment"),
            TokenKind::Dot => write!(f, "'.'"),
            TokenKind::Comma => write!(f, "','"),
            TokenKind::Colon => write!(f, "':'"),
            TokenKind::Semicolon => write!(f, "';'"),
            TokenKind::Equal => write!(f, "'='"),
            TokenKind::FatArrow => write!(f, "'=>'"),
            TokenKind::LBrace => write!(f, "'{{'"),
            TokenKind::RBrace => write!(f, "'}}'"),
            TokenKind::LParen => write!(f, "'('"),
            TokenKind::RParen => write!(f, "')'"),
            TokenKind::LBracket => write!(f, "'['"),
            TokenKind::RBracket => write!(f, "']'"),
            TokenKind::Identifier(s) => write!(f, "identifier '{s}'"),
            TokenKind::DotIdentifier(s) => write!(f, "field '.{s}'"),
            TokenKind::StringLiteral(s) => write!(f, "string \"{s}\""),
            TokenKind::CharLiteral(c) => write!(f, "char '{c}'"),
            TokenKind::NumberLiteral(n) => write!(f, "number '{n}'"),
            TokenKind::BoolLiteral(b) => write!(f, "bool '{b}'"),
            TokenKind::NullLiteral => write!(f, "null"),
        }
    }
}
