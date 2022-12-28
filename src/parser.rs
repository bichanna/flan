use crate::ast::Node;
use crate::error::ParserError;
use crate::token::{Token, TokenType};

pub struct Parser {
    c: usize,
    current: Token,
    errors: Vec<ParserError>,
    statements: Vec<Node>,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            c: 0,
            current: Token::new(TokenType::EOF, String::new(), 0, 0),
            errors: vec![],
            statements: vec![],
        }
    }

    /// Checks if the current token is in the given types
    fn does_match(&mut self, these: &[TokenType], tokens: &Vec<Token>) -> bool {
        for kind in these {
            if self.check_current(*kind, tokens) {
                self.advance(tokens);
                return true;
            }
        }
        false
    }

    /// Checks whether the current token is the expected type or not, and if not, adds an error
    fn expect(&mut self, kind: TokenType, message: &str, tokens: &Vec<Token>) {
        if self.check_current(kind, tokens) {
            self.advance(tokens);
        } else {
            self.add_error(message);
        }
    }

    /// Advances one token
    fn advance(&mut self, tokens: &Vec<Token>) {
        if !self.is_end(tokens) {
            self.c += 1;
            self.current = tokens[self.c].clone();
        } else {
            self.current = tokens[tokens.len()].clone();
        }
    }

    /// Checks if the token type of the current token is the same as the expected token type
    fn check_current(&self, kind: TokenType, tokens: &Vec<Token>) -> bool {
        if tokens[self.c].clone().kind == kind {
            true
        } else {
            false
        }
    }

    /// Checks if the token type of the next token is the same as the expected token type
    fn check_next(&self, kind: TokenType, tokens: &Vec<Token>) -> bool {
        if self.is_end(tokens) {
            false
        } else {
            if tokens[self.c].clone().kind == kind {
                true
            } else {
                false
            }
        }
    }

    /// Returns the previous token
    fn previous(&self, tokens: &Vec<Token>) -> Token {
        if self.c == 0 {
            tokens[0].clone()
        } else {
            tokens[self.c - 1].clone()
        }
    }

    /// Checks if the end is reached
    fn is_end(&self, tokens: &Vec<Token>) -> bool {
        match tokens[self.c].kind {
            TokenType::EOF => true,
            _ => false,
        }
    }

    /// Appends the error created with the given error message and the current line and column
    fn add_error(&mut self, message: &str) {
        let error = ParserError::new(message, self.current.position.0, self.current.position.1);
        self.errors.push(error);
    }
}
