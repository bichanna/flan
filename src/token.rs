use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    Num,        // Number
    Str,        // String
    Id,         // Identifier
    Atom,       // Atom
    Underscore, // Underscore
    LParen,     // (
    RParen,     // )
    LBrace,     // {
    RBrace,     // }
    LBracket,   // [
    RBracket,   // ]
    Colon,      // :
    SColon,     // ;
    ColonEq,    // :=
    Plus,       // +
    DPlus,      // ++
    PlusEq,     // +=
    Minus,      // -
    MinusGT,    // ->
    DMinus,     // --
    MinusEq,    // -=
    Mul,        // *
    MulEq,      // *=
    Div,        // /
    DivEq,      // /=
    Mod,        // %
    ModEq,      // %=
    At,         // @
    Caret,      // ^
    Comma,      // ,
    RPipe,      // |>
    LPipe,      // <|
    GT,         // >
    LT,         // <
    GTEq,       // >=
    LTEq,       // <=
    Bang,       // !
    BangEq,     // !=
    DEq,        // ==
    Equal,      // =
    Dot,        // .
    DAmp,       // &&
    DPipe,      // ||

    Func,   // func
    Else,   // else
    Match,  // match
    Or,     // or
    And,    // and
    True,   // true
    False,  // false
    Null,   // null
    Import, // import

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

impl Token {
    pub fn print(&self) -> String {
        if self.value == "" {
            format!("{:?}", self.kind)
        } else if self.kind == TokenType::Atom {
            format!(":{}", self.value)
        } else {
            format!("{}", self.value)
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}:{}", self.kind, self.value)
    }
}
