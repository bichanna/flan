use crate::error::Position;

use std::sync::Arc;

/// Token for the lexer
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenType,
    pub pos: Position,
}

impl TokenType {
    /// Returns the TokenType of the keyword if the given &str is a keyword
    pub fn get_type(value: &str) -> Option<TokenType> {
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
            "case" => Some(TokenType::Case),
            "then" => Some(TokenType::Then),
            "import" => Some(TokenType::Import),
            "recover" => Some(TokenType::Recover),
            "panic" => Some(TokenType::Panic),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Int(i64),
    Float(f64),
    Str(String),
    Atom(Arc<str>),
    Id(Arc<str>),
    Empty,    // _
    LParen,   // (
    RParen,   // )
    SLBrace,  // s{
    ILBrace,  // i{
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]
    ColonEq,  // :=
    Plus,     // +
    PlusEq,   // +=
    Minus,    // -
    MinusGT,  // ->
    MinusEq,  // -=
    Mult,     // *
    MultEq,   // *=
    Div,      // /
    DivEq,    // /=
    Mod,      // %
    ModEq,    // %=
    Comma,    // ,
    BarGT,    // |>
    BarLT,    // <|
    Tilde,    // ~
    LTilde,   // <~
    GT,       // >
    LT,       // <
    GTEq,     // >=
    LTEq,     // <=
    Bang,     // !
    BangEq,   // !=
    Equal,    // =
    DoubleEq, // ==
    Dot,      // .
    Ellipsis, // ...
    DotEq,    // ..=
    LDot,     // ..<

    Func,    // fn
    If,      // if
    Where,   // where
    Match,   // match
    Then,    // then
    Case,    // case
    And,     // and
    Or,      // or
    Else,    // else
    True,    // true
    Not,     // not
    False,   // false
    Import,  // import
    Recover, // recover
    Panic,   // panic

    Eof, // end of file
}
