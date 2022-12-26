use crate::error::ParserError;
use crate::token::Token;

pub struct Lexer {
    errors: Vec<ParserError>,
    source: String,
    tokens: Vec<Token>,
    line: usize,
    col: usize,
    c: usize,
    current: char,
}

impl Lexer {
    pub fn new(source: String) -> Self {
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
}
