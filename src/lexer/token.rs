use crate::error::Position;

use std::sync::Arc;

/// Token for the lexer
#[derive(Clone)]
pub struct Token {
    pub kind: TokenType,
    pub pos: Position,
}

#[derive(Clone, PartialEq)]
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

    Func,   // fn
    If,     // if
    Where,  // where
    Match,  // match
    Then,   // then
    And,    // and
    Or,     // or
    Else,   // else
    True,   // true
    Not,    // not
    False,  // false
    Import, // import

    EOF, // end of file
}
