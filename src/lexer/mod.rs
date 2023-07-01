pub mod token;

use super::error::{flan_panic_exit, ErrType, Position, Stack};
use super::lexer::token::{Token, TokenType};
use super::util::PrevPeekable;

use std::fs::File;
use std::io::Read;
use std::str::Chars;
use std::sync::Arc;

/// Tokenizes a source code
struct Lexer<'a> {
    /// The path index of the source being tokenized
    path_idx: usize,
    /// Tokens
    tokens: Vec<Token>,
    /// An iterator over the source
    chars: PrevPeekable<Chars<'a>>,
    /// Current line number
    line: usize,
    /// Current column number
    col: usize,
    /// Current character
    current: char,
    /// Character counter
    c: usize,
}

impl<'a> Lexer<'a> {
    /// The public interface for the lexer
    pub fn get_tokens(path: &str) -> Vec<Token> {
        Stack::add_path(path);

        let file = File::open(path);
        if file.is_err() {
            flan_panic_exit(&format!("could not open {}", path), 1);
        }

        // getting the contents of the file
        let mut contents = String::new();
        if file.unwrap().read_to_string(&mut contents).is_err() {
            flan_panic_exit(&format!("could not read {}", path), 1);
        }

        // converting the contents of the file into Chars
        let mut chars = PrevPeekable::new(contents.chars());
        let current_char = chars.next();

        // it's an empty file!
        if current_char.is_none() {
            std::process::exit(0);
        }

        let mut lexer = Lexer {
            path_idx: Stack::last_path_index(),
            tokens: vec![],
            line: 1,
            col: 1,
            chars,
            current: current_char.unwrap(),
            c: 0,
        };

        lexer.tokenize();
        lexer.tokens
    }

    /// Tokenizes the source
    fn tokenize(&mut self) {
        let mut revert = false;
        while !self.is_end() {
            match self.current {
                n if n.is_whitespace() => {}
                '(' => self.append(TokenType::LParen),
                ')' => self.append(TokenType::RParen),
                '{' => self.append(TokenType::LBrace),
                '}' => self.append(TokenType::RBrace),
                '[' => self.append(TokenType::LBracket),
                ']' => self.append(TokenType::RBracket),
                ',' => self.append(TokenType::Comma),
                '~' => self.append(TokenType::Tilde),
                ':' => match self.peek() {
                    '=' => self.append_and_advance(TokenType::ColonEq),
                    _ => self.report_error(&format!("expected ':=' but got '{}'", self.current)),
                },
                '|' => match self.peek() {
                    '>' => self.append_and_advance(TokenType::BarGT),
                    _ => self.report_error(&format!("expected '|>' but got '{}'", self.current)),
                },
                '+' => match self.peek() {
                    '=' => self.append_and_advance(TokenType::PlusEq),
                    _ => self.append(TokenType::Plus),
                },
                '-' => match self.peek() {
                    '=' => self.append_and_advance(TokenType::MinusEq),
                    _ => self.append(TokenType::Minus),
                },
                '*' => match self.peek() {
                    '=' => self.append_and_advance(TokenType::MultEq),
                    _ => self.append(TokenType::Mult),
                },
                '/' => match self.peek() {
                    '=' => self.append_and_advance(TokenType::DivEq),
                    _ => self.append(TokenType::Div),
                },
                '%' => match self.peek() {
                    '=' => self.append_and_advance(TokenType::ModEq),
                    _ => self.append(TokenType::Mod),
                },
                '<' => match self.peek() {
                    '=' => self.append_and_advance(TokenType::LTEq),
                    '~' => self.append_and_advance(TokenType::LTilde),
                    '|' => self.append_and_advance(TokenType::BarLT),
                    _ => self.append(TokenType::LT),
                },
                '>' => match self.peek() {
                    '=' => self.append_and_advance(TokenType::GTEq),
                    _ => self.append(TokenType::GT),
                },
                '!' => match self.peek() {
                    '=' => self.append_and_advance(TokenType::BangEq),
                    _ => self.append(TokenType::Bang),
                },
                '=' => match self.peek() {
                    '=' => self.append_and_advance(TokenType::DoubleEq),
                    _ => self.append(TokenType::Equal),
                },
                '.' => match self.peek() {
                    '.' => {
                        self.advance();
                        match self.peek() {
                            '.' => self.append_and_advance(TokenType::Ellipsis),
                            '<' => self.append_and_advance(TokenType::LDot),
                            '=' => self.append_and_advance(TokenType::DotEq),
                            _ => {
                                let err_char = self.peek();
                                self.report_error(&format!(
                                    "expected '...', '..<', or '..=' but got '..{}'",
                                    err_char
                                ));
                            }
                        }
                    }
                    _ => self.append(TokenType::Dot),
                },
                '#' => {
                    // one-line comment
                    while self.current != '\n' && !self.is_end() {
                        self.advance();
                    }
                }
                n if n.is_alphabetic() || n == '_' => {
                    // identifier or keyword
                    let value = self.build_str(|l| l.current.is_alphanumeric() || l.current == '_');

                    if value == "s" && self.current == '{' {
                        self.append_and_advance(TokenType::SLBrace);
                    } else if value == "i" && self.current == '}' {
                        self.append_and_advance(TokenType::ILBrace);
                    } else {
                        if let Some(keyword) = Self::keyword(&value) {
                            self.append(keyword);
                        } else {
                            self.append(TokenType::Id(Arc::from(value.as_str())));
                        }
                        revert = true;
                    }
                }
                n if n.is_ascii_digit() => {
                    // number
                    if self.current == '0' && self.peek() == 'x' {
                        // hex number
                        self.advance();
                        self.advance();
                        let hex = self.build_str(|l| l.current.is_ascii_hexdigit());

                        if let Ok(num) = i64::from_str_radix(&hex, 16) {
                            self.append(TokenType::Int(num));
                        } else {
                            self.report_error("invalid hexadecimal number");
                        }

                        revert = false;
                    } else {
                        // integer or float
                        let mut num = String::new();
                        let mut has_dot = false;

                        while !self.is_end() && self.current.is_ascii_digit() {
                            num.push(self.current);
                            self.advance();
                            if self.current == '.' && self.peek().is_ascii_digit() {
                                if has_dot {
                                    self.report_error("unexpected '.'");
                                } else {
                                    num.push('.');
                                    has_dot = true;
                                    self.advance();
                                }
                            }
                        }

                        if has_dot {
                            if let Ok(float) = str::parse::<f64>(&num) {
                                self.append(TokenType::Float(float));
                            } else {
                                self.report_error("invalid floating point number");
                            }
                        } else if let Ok(int) = str::parse::<i64>(&num) {
                            self.append(TokenType::Int(int));
                        } else {
                            self.report_error("invalid integer");
                        }

                        revert = true;
                    }
                }
                '"' => {
                    // string
                    self.advance();
                    let value = self.build_str(|l| l.current != '"');
                    self.append(TokenType::Str(value));
                }
                _ => self.report_error(&format!("unexpected character: '{}'", self.current)),
            }
            if !revert {
                self.advance();
            }
        }
        self.append(TokenType::EOF);
    }

    /// Checks if the lexer is at the end of the source or not
    fn is_strict_end(&mut self) -> bool {
        self.is_end() || self.chars.peek().is_none()
    }

    /// Checks if the lexer is at the end of the source or not
    fn is_end(&mut self) -> bool {
        self.chars.current.is_none()
    }

    /// Advances on character forward
    fn advance(&mut self) {
        if let Some(next_char) = self.chars.next() {
            self.c += 1;
            if std::mem::replace(&mut self.current, next_char) == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        } else {
            self.current = '\0';
        }
    }

    /// Returns the next character without moving the iterator forward
    fn peek(&mut self) -> char {
        if let Some(next_char) = self.chars.peek() {
            *next_char
        } else {
            '\0'
        }
    }

    /// Returns the TokenType of the keyword if the given &str is a keyword
    fn keyword(value: &str) -> Option<TokenType> {
        match value.to_lowercase().as_str() {
            "if" => Some(TokenType::If),
            "fn" => Some(TokenType::Func),
            "match" => Some(TokenType::Match),
            "or" => Some(TokenType::Or),
            "and" => Some(TokenType::And),
            "not" => Some(TokenType::Not),
            "true" => Some(TokenType::True),
            "false" => Some(TokenType::False),
            "else" => Some(TokenType::Else),
            "where" => Some(TokenType::Where),
            "then" => Some(TokenType::Then),
            "import" => Some(TokenType::Import),
            _ => None,
        }
    }

    /// Appends a `Token` to `tokens`
    fn append(&mut self, kind: TokenType) {
        let token = Token {
            kind,
            pos: self.current_position(),
        };
        self.tokens.push(token);
    }

    /// Appends a `Token` to `tokens` and advances the `chars` iterator forward
    fn append_and_advance(&mut self, kind: TokenType) {
        self.append(kind);
        self.advance();
    }

    /// Returns the current position as `Position`
    fn current_position(&self) -> Position {
        (self.col, self.line)
    }

    /// Builds a `String` with the given condition
    fn build_str(&mut self, func: fn(&mut Self) -> bool) -> String {
        let mut builder = String::new();
        while !self.is_end() && func(self) {
            builder.push(self.current);
            self.advance();
        }
        builder
    }

    /// Reports a syntax error
    fn report_error(&self, msg: &str) {
        Stack::new(
            ErrType::Syntax,
            msg.to_string(),
            self.current_position(),
            self.path_idx,
        )
        .report(1);
    }
}
