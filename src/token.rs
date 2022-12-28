use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    Num,      // Number
    Str,      // String
    Id,       // Identifier
    LParen,   // (
    RParen,   // )
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]
    Colon,    // :
    SColon,   // ;
    Plus,     // +
    DPlus,    // ++
    PlusEq,   // +=
    Minus,    // -
    DMinus,   // --
    MinusEq,  // -=
    Mul,      // *
    MulEq,    // *=
    Div,      // /
    DivEq,    // /=
    Mod,      // %
    ModEq,    // %=
    At,       // @
    Caret,    // ^
    Comma,    // ,
    RPipe,    // |>
    LPipe,    // <|
    GT,       // >
    LT,       // <
    GTEq,     // >=
    LTEq,     // <=
    Bang,     // !
    BangEq,   // !=
    DEq,      // ==
    Equal,    // =
    Dot,      // .
    DAmp,     // &&
    DPipe,    // ||

    Func,     // fn
    Class,    // class
    Static,   // static
    Var,      // let
    Const,    // const
    If,       // if
    Else,     // else
    For,      // for
    While,    // while
    Super,    // super
    This,     // this
    Return,   // return
    Continue, // continue
    Break,    // break
    True,     // true
    False,    // false
    Null,     // null
    Import,   // import

    EOF,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenType,
    pub value: String,
    pub position: (usize, usize), // (line, col)
}

impl Token {
    pub fn new(kind: TokenType, value: String, line: usize, col: usize) -> Self {
        Token {
            kind,
            value,
            position: (line, col),
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}:{}", self.kind, self.value)
    }
}
