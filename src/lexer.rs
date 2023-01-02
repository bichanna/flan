use std::process;

use crate::error::ParserError;
use crate::token::{Token, TokenType};

pub struct Lexer<'a> {
    pub tokens: Vec<Token>,
    errors: Vec<ParserError>,
    source: &'a String,
    line: usize,
    col: usize,
    c: usize,
    current: char,
}

impl<'a> Lexer<'a> {
    pub fn new<'b>(source: &'a String) -> Self {
        Lexer {
            errors: vec![],
            source,
            tokens: vec![],
            line: 1,
            col: 1,
            c: 0,
            current: ' ',
        }
    }

    /// Reports errors if any
    pub fn report_errors(&self, filename: &str) {
        if self.errors.len() > 0 {
            for err in &self.errors {
                println!("{}", err.format(filename));
                println!(
                    "{}",
                    self.source.split("\n").collect::<Vec<&str>>()[err.line]
                );
            }
            process::exit(1);
        }
    }

    /// Tokenizes the source
    pub fn tokenize(&mut self) {
        self.current = self.source.chars().nth(self.c).unwrap();

        while !self.is_end() {
            match self.current {
                '\n' => {
                    self.line += 1;
                    self.col = 1;
                }
                '\t' | ' ' => {}
                '(' => self.add_no_value_token(TokenType::LParen),
                ')' => self.add_no_value_token(TokenType::RParen),
                '{' => self.add_no_value_token(TokenType::LBrace),
                '}' => self.add_no_value_token(TokenType::RBrace),
                '[' => self.add_no_value_token(TokenType::LBracket),
                ']' => self.add_no_value_token(TokenType::RBracket),
                ':' => self.add_no_value_token(TokenType::Colon),
                ';' => self.add_no_value_token(TokenType::SColon),
                '@' => self.add_no_value_token(TokenType::At),
                '^' => self.add_no_value_token(TokenType::Caret),
                ',' => self.add_no_value_token(TokenType::Comma),
                '.' => self.add_no_value_token(TokenType::Dot),
                '+' => match self.next_char() {
                    '+' => {
                        self.add_no_value_token(TokenType::DPlus);
                        self.advance();
                    }
                    '=' => {
                        self.add_no_value_token(TokenType::PlusEq);
                        self.advance();
                    }
                    _ => self.add_no_value_token(TokenType::Plus),
                },
                '-' => match self.next_char() {
                    '-' => {
                        self.add_no_value_token(TokenType::DMinus);
                        self.advance();
                    }
                    '=' => {
                        self.add_no_value_token(TokenType::MinusEq);
                        self.advance();
                    }
                    _ => self.add_no_value_token(TokenType::Minus),
                },
                '*' => match self.next_char() {
                    '=' => {
                        self.add_no_value_token(TokenType::MulEq);
                        self.advance();
                    }
                    _ => self.add_no_value_token(TokenType::Mul),
                },
                '/' => match self.next_char() {
                    '/' => {
                        // one-line comment
                        while self.current != '\n' {
                            self.advance();
                        }
                    }
                    '*' => {
                        // multi-line comment
                        self.advance();
                        self.advance();
                        self.skip_block_comment();
                    }
                    '=' => {
                        self.add_no_value_token(TokenType::DivEq);
                        self.advance();
                    }
                    _ => self.add_no_value_token(TokenType::Div),
                },
                '%' => match self.next_char() {
                    '=' => {
                        self.add_no_value_token(TokenType::ModEq);
                        self.advance();
                    }
                    _ => self.add_no_value_token(TokenType::Mod),
                },
                '|' => match self.next_char() {
                    '>' => {
                        self.add_no_value_token(TokenType::RPipe);
                        self.advance();
                    }
                    '|' => {
                        self.add_no_value_token(TokenType::DPipe);
                        self.advance();
                    }
                    _ => {
                        self.advance();
                        self.add_error("unrecognized character");
                    }
                },
                '<' => match self.next_char() {
                    '|' => {
                        self.add_no_value_token(TokenType::LPipe);
                        self.advance();
                    }
                    '=' => {
                        self.add_no_value_token(TokenType::LTEq);
                        self.advance();
                    }
                    _ => self.add_no_value_token(TokenType::LT),
                },
                '>' => match self.next_char() {
                    '=' => {
                        self.add_no_value_token(TokenType::GTEq);
                        self.advance();
                    }
                    _ => self.add_no_value_token(TokenType::GT),
                },
                '!' => match self.next_char() {
                    '=' => {
                        self.add_no_value_token(TokenType::BangEq);
                        self.advance();
                    }
                    _ => self.add_no_value_token(TokenType::Bang),
                },
                '=' => match self.next_char() {
                    '=' => {
                        self.add_no_value_token(TokenType::DEq);
                        self.advance();
                    }
                    _ => self.add_no_value_token(TokenType::Equal),
                },
                '&' => match self.next_char() {
                    '&' => {
                        self.add_no_value_token(TokenType::DAmp);
                        self.advance();
                    }
                    _ => {
                        self.advance();
                        self.add_error("unrecognized character");
                    }
                },
                _ => {
                    if self.current.is_alphabetic() || self.current == '_' {
                        // an identifier or a keyword
                        let mut var = String::new();

                        if (self.current.is_alphabetic() || self.current == '_')
                            && !self.current.is_numeric()
                        {
                            var.push(self.current);
                            self.advance();
                        }

                        while !self.is_end()
                            && (self.current.is_alphanumeric() || self.current == '_')
                        {
                            var.push(self.current);
                            self.advance();
                        }

                        // include ! if there's any
                        if self.current == '!' {
                            var.push(self.current);
                            self.advance();
                        }

                        match Lexer::keyword(var.as_str()) {
                            Some(kind) => self.add_no_value_token(kind),
                            _ => self.add_token(TokenType::Id, var),
                        }
                        self.reverse();
                    } else if self.current.is_numeric() {
                        // a number
                        let mut number = String::new();

                        if self.current.is_numeric() {
                            let mut had_dot = false;

                            while !self.is_end() && self.current.is_numeric() {
                                number.push(self.current);
                                self.advance();

                                if self.current == '.' && self.next_char().is_numeric() {
                                    if had_dot {
                                        self.add_error("invalid dot");
                                    } else {
                                        number.push('.');
                                        had_dot = true;
                                    }
                                }
                            }
                        }
                        if self.current == '0' && self.next_char() == 'x' {
                            // hex number
                            self.advance();
                            self.advance();
                            while !self.is_end() && self.current.is_ascii_hexdigit() {
                                number.push(self.current);
                            }
                        }

                        self.add_token(TokenType::Num, number);
                        self.reverse();
                    } else if self.current == '"' {
                        // a string
                        let mut value = String::new();
                        self.advance();

                        while !self.is_end() && self.current != '"' {
                            if self.current == '\\' {
                                // excape chars
                                self.advance();
                                match self.current {
                                    '0' => value.push('\0'),
                                    '"' => value.push('"'),
                                    '\\' => value.push('\\'),
                                    '%' => value.push('%'),
                                    'n' => value.push('\n'),
                                    'r' => value.push('\r'),
                                    't' => value.push('\t'),
                                    c => value.push(c),
                                };
                            } else {
                                if self.current == '\n' {
                                    self.line += 1;
                                    self.col = 1;
                                }
                                value.push(self.current);
                            }
                            self.advance();
                        }

                        self.add_token(TokenType::Str, value);
                    }
                }
            };
            self.advance();
        }
        self.add_no_value_token(TokenType::EOF);
    }

    /// Skips the rest of a block comment
    fn skip_block_comment(&mut self) {
        let mut nesting = 1;
        while nesting > 0 {
            if self.current == '\n' {
                self.line += 1;
                self.col = 1;
            } else if self.is_end() {
                self.add_error("an unterminated block comment");
                break;
            } else if self.current == '*' && self.next_char() == '/' {
                self.advance();
                self.advance();
                nesting -= 1;
            } else if self.current == '/' && self.next_char() == '*' {
                self.advance();
                self.advance();
                nesting += 1;
            }
            self.advance();
        }
    }

    /// Appends the Token created with the given TokenType without any String value
    fn add_no_value_token(&mut self, kind: TokenType) {
        self.add_token(kind, String::new());
    }

    /// Appends the Token created with the given TokenType with a String value
    fn add_token(&mut self, kind: TokenType, value: String) {
        let token = Token::new(kind, value, self.line, self.col);
        self.tokens.push(token);
    }

    /// Returns the next character without advancing
    fn next_char(&self) -> char {
        if self.is_end() {
            '\0'
        } else {
            self.source.chars().nth(self.c + 1).unwrap()
        }
    }

    /// Returns the TokenType of the keyword if the given &str is a keyword
    fn keyword(value: &str) -> Option<TokenType> {
        match value.to_lowercase().as_str() {
            "fn" => Some(TokenType::Func),
            "method" => Some(TokenType::Method),
            "class" => Some(TokenType::Class),
            "static" => Some(TokenType::Static),
            "let" => Some(TokenType::Var),
            "const" => Some(TokenType::Const),
            "if" => Some(TokenType::If),
            "else" => Some(TokenType::Else),
            "for" => Some(TokenType::For),
            "in" => Some(TokenType::In),
            "while" => Some(TokenType::While),
            "super" => Some(TokenType::Super),
            "this" => Some(TokenType::This),
            "return" => Some(TokenType::Return),
            "continue" => Some(TokenType::Continue),
            "break" => Some(TokenType::Break),
            "true" => Some(TokenType::True),
            "false" => Some(TokenType::False),
            "null" => Some(TokenType::Null),
            "import" => Some(TokenType::Import),
            _ => None,
        }
    }

    /// Appends the error created with the given error message and the current line and column
    fn add_error(&mut self, msg: &str) {
        let error = ParserError::new(msg, self.line, self.col);
        self.errors.push(error);
    }

    /// Checks if the lexer is at the end of the source or not
    fn is_end(&self) -> bool {
        if self.source.len() <= self.c || self.source.len() <= self.c + 1 {
            true
        } else {
            false
        }
    }

    /// Advances one character
    fn advance(&mut self) -> char {
        if !self.is_end() {
            if self.current == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.c += 1;
            self.current = self.source.chars().nth(self.c).unwrap();
        } else {
            self.c = self.source.len();
        }
        self.current
    }

    /// Revereses one character
    fn reverse(&mut self) -> char {
        if self.c - 1 > 0 {
            self.c -= 1;
            self.current = self.source.chars().nth(self.c).unwrap();
            if self.current == '\n' {
                self.line -= 1;
                let mut count: i32 = -1;
                let mut i = self.c - 1;
                while self.source.chars().nth(i).unwrap() != '\n' {
                    count += 1;
                    i += 1;
                }
                self.col = count as usize;
            } else {
                self.col -= 1;
            }
        } else {
            self.c = 0;
            self.current = self.source.chars().nth(0).unwrap();
        }
        self.current
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer() {
        let source = r#"
let name! = "Nobuharu Shimazu";
let _age = 16;
println(name!, _age);
// Some comment
/* comment!! /* block */ */
}"#;
        let source = &String::from(source);
        let mut lexer = Lexer::new(source);
        lexer.tokenize();

        assert_eq!(lexer.errors.len(), 0);
        assert_eq!(lexer.tokens.len(), 18);
    }
}
