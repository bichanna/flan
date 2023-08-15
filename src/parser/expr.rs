use crate::error::Position;
use crate::lexer::token::Token;

use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum CallArgType {
    Positional,
    Unpacking,
}

#[derive(Debug, Clone)]
pub struct CallArg {
    pub kind: CallArgType,
    pub expr: Box<Expr>,
    pub mutable: bool,
}

#[derive(Debug, Clone)]
pub struct MatchBranch {
    pub case: Box<Expr>,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct WhenBranch {
    pub cond: Box<Expr>,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Binary {
        left: Box<Expr>,
        right: Box<Expr>,
        op: Token,
    },
    Group(Box<Expr>),
    Unary {
        right: Box<Expr>,
        op: Token,
    },
    Logic {
        left: Box<Expr>,
        right: Box<Expr>,
        op: Token,
    },
    Var {
        name: Arc<str>,
        pos: Position,
    },
    Assign {
        init: bool,
        mutable: bool,
        left: Box<Expr>,
        right: Box<Expr>,
        pos: Position,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<CallArg>,
        pos: Position,
    },
    Get {
        inst: Box<Expr>,
        attr: Box<Expr>,
        pos: Position,
    },
    Set {
        inst: Box<Expr>,
        attr: Box<Expr>,
        val: Box<Expr>,
        pos: Position,
    },
    Func {
        name: Option<Arc<str>>,
        params: Vec<(Token, bool)>,
        rest: Option<(Token, bool)>,
        body: Box<Expr>,
        pos: Position,
    },
    Match {
        cond: Box<Expr>,
        branches: Vec<MatchBranch>,
        pos: Position,
    },
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        els: Option<Box<Expr>>,
        pos: Position,
    },
    When {
        branches: Vec<WhenBranch>,
        pos: Position,
    },
    Import {
        exprs: Vec<Expr>,
        pos: Position,
    },
    Block {
        exprs: Vec<Expr>,
        pos: Position,
    },
    Str {
        mutable: bool,
        val: String,
        pos: Position,
    },
    Atom {
        val: Arc<str>,
        pos: Position,
    },
    Int {
        val: i64,
        pos: Position,
    },
    Float {
        val: f64,
        pos: Position,
    },
    Bool {
        val: bool,
        pos: Position,
    },
    Empty(Position),
    List {
        mutable: bool,
        elems: Vec<Expr>,
        pos: Position,
    },
    Tuple {
        elems: Box<[Expr]>,
        pos: Position,
    },
    Obj {
        mutable: bool,
        keys: Vec<Token>,
        vals: Vec<Expr>,
        pos: Position,
    },
    Nil(Position),
    Panic {
        expr: Box<Expr>,
        pos: Position,
    },
    Recover {
        recoveree: Box<Expr>,
        body: Box<Expr>,
        pos: Position,
    },
}
