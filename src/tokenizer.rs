use crate::ast::{Token, TokenType};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct Tokenizer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
}

impl Tokenizer {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut all_chars = Vec::new();
        for (_idx, line) in reader.lines().enumerate() {
            let line = line?;
            all_chars.extend(line.chars());
            all_chars.push('\n');
        }
        Ok(Self {
            chars: all_chars,
            pos: 0,
            line: 1,
            column: 1,
        })
    }

    fn peek(&self) -> char {
        self.chars.get(self.pos).cloned().unwrap_or('\0')
    }

    fn peek_next(&self) -> char {
        self.chars.get(self.pos + 1).cloned().unwrap_or('\0')
    }

    fn consume(&mut self) -> char {
        let c = self.peek();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else if c != '\0' {
            self.column += 1;
        }
        self.pos += 1;
        c
    }

    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.chars.len() {
            let c = self.peek();
            if c.is_whitespace() && c != '\n' {
                self.consume();
            } else if c == '#' {
                // Skip comment line
                while self.pos < self.chars.len() && self.peek() != '\n' {
                    self.consume();
                }
            } else {
                break;
            }
        }
    }

    pub fn get_lines_until_end_block(&mut self) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        while self.pos < self.chars.len() {
            let c = self.peek();
            if c == '@' && self.peek_next() == '}' {
                // Don't consume yet, the next getToken will handle it or we skip it here
                break;
            }
            if c == '\n' {
                self.consume();
                lines.push(current_line);
                current_line = String::new();
            } else {
                current_line.push(self.consume());
            }
        }
        lines
    }

    fn get_embed_code(&mut self) -> String {
        self.skip_whitespace_and_comments();
        if self.peek() == '@' && self.peek_next() == '{' {
            self.consume(); // '@'
            self.consume(); // '{'
            let mut code = String::new();
            while self.pos < self.chars.len() {
                if self.peek() == '@' && self.peek_next() == '}' {
                    self.consume(); // '@'
                    self.consume(); // '}'
                    break;
                }
                code.push(self.consume());
            }
            code
        } else {
            String::new()
        }
    }

    pub fn get_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();
        
        let start_pos = self.pos;
        let start_line = self.line;
        let start_col = self.column;

        let c = self.peek();
        if c == '\0' {
            return Token {
                token_type: TokenType::TEof,
                content: "".to_string(),
                span: crate::ast::Span::new(start_pos, start_pos, start_line, start_col),
                embed_code: "".to_string(),
            };
        }

        let mut token = Token::default();
        token.span.start = start_pos;
        token.span.line = start_line;
        token.span.column = start_col;

        if c.is_alphabetic() {
            let mut s = String::new();
            while self.peek().is_alphanumeric() || self.peek() == '-' || self.peek() == '_' {
                let mut next_c = self.consume();
                if next_c == '-' { next_c = '_'; } 
                s.push(next_c);
            }
            token.token_type = TokenType::TRuleName;
            token.content = s;
        } else if c == '"' {
            self.consume(); // '"'
            let mut s = String::new();
            while self.peek() != '"' && self.pos < self.chars.len() {
                s.push(self.consume());
            }
            self.consume(); // '"'
            token.token_type = TokenType::TCharVal;
            token.content = s;
        } else if c == '%' {
            self.consume(); // '%'
            let val_type = self.peek();
            self.consume(); // b, d, x
            let mut s = String::new();
            match val_type {
                'b' => {
                    while "01.-".contains(self.peek()) {
                        s.push(self.consume());
                    }
                    token.token_type = TokenType::TBinVal;
                }
                'd' => {
                    while self.peek().is_ascii_digit() || ".--".contains(self.peek()) {
                        s.push(self.consume());
                    }
                    token.token_type = TokenType::TDecVal;
                }
                'x' => {
                    while self.peek().is_ascii_hexdigit() || ".-".contains(self.peek()) {
                        s.push(self.consume());
                    }
                    token.token_type = TokenType::THexVal;
                }
                _ => {
                    token.token_type = TokenType::TError;
                }
            }
            token.content = s;
        } else if c == '<' {
            self.consume(); // '<'
            let mut s = String::new();
            while self.peek() != '>' && self.pos < self.chars.len() {
                s.push(self.consume());
            }
            self.consume(); // '>'
            token.token_type = TokenType::TCharVal; // C# mapping proseval to charval in Element()
            token.content = s;
        } else if c == '|' {
            self.consume();
            token.token_type = TokenType::TAlternative;
            token.content = "|".to_string();
        } else if c == '(' {
            self.consume();
            token.token_type = TokenType::TGroupLeft;
            token.content = "(".to_string();
        } else if c == ')' {
            self.consume();
            token.token_type = TokenType::TGroupRight;
            token.content = ")".to_string();
        } else if c == '[' {
            self.consume();
            token.token_type = TokenType::TOptionLeft;
            token.content = "[".to_string();
        } else if c == ']' {
            self.consume();
            token.token_type = TokenType::TOptionRight;
            token.content = "]".to_string();
        } else if c == '=' {
            self.consume();
            token.token_type = TokenType::TEqual;
            token.content = "=".to_string();
        } else if c == '*' {
            self.consume();
            token.token_type = TokenType::TAsterisk;
            token.content = "*".to_string();
        } else if c == '+' {
            self.consume();
            token.token_type = TokenType::TPlus;
            token.content = "+".to_string();
        } else if c == '&' {
            self.consume();
            token.token_type = TokenType::TAmpersand;
            token.content = "&".to_string();
        } else if c == '.' {
            self.consume();
            token.token_type = TokenType::TPiriod;
            token.content = ".".to_string();
        } else if c == '\n' {
            self.consume();
            token.token_type = TokenType::TNl;
            token.content = "\n".to_string();
        } else if c == '@' {
            self.consume(); // '@'
            let next = self.peek();
            if next == '@' {
                self.consume();
                token.token_type = TokenType::TBody;
                token.content = "@@".to_string();
            } else if next == '{' {
                self.consume();
                token.token_type = TokenType::TUserDefineLeft;
                token.content = "@{".to_string();
            } else if next == '}' {
                self.consume();
                token.token_type = TokenType::TUserDefineRight;
                token.content = "@}".to_string();
            } else {
                let mut kw = String::new();
                while self.peek().is_alphanumeric() || self.peek() == '-' || self.peek() == '_' {
                    kw.push(self.consume());
                }
                match kw.as_str() {
                    "class" => token.token_type = TokenType::TClass,
                    "token" => token.token_type = TokenType::TToken,
                    "namespace" => token.token_type = TokenType::TNamespace,
                    "decl" => token.token_type = TokenType::TDecl,
                    _ => token.token_type = TokenType::TError,
                }
                token.content = kw;
            }
        } else {
            self.consume();
            token.token_type = TokenType::TError;
        }

        // Process embed code
        token.embed_code = self.get_embed_code();
        
        token.span.end = self.pos;
        token
    }
}
