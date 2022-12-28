use crate::token::{Token, TokenType};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Binary {
        left: Box<Expr>,
        right: Box<Expr>,
        op: Token,
    },
    Group {
        expr: Box<Expr>,
    },
    Unary {
        right: Box<Expr>,
        op: Token,
    },
    Literal {
        kind: TokenType,
        value: String,
    },
    Logical {
        left: Box<Expr>,
        right: Box<Expr>,
        op: Token,
    },
    Variable {
        name: Token,
    },
    Assign {
        name: Token,
        value: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Box<Expr>>,
        token: Token,
    },
    Get {
        instance: Box<Expr>,
        token: Token,
    },
    Set {
        instance: Box<Expr>,
        token: Token,
        value: Box<Expr>,
    },
    Super {
        token: Token,
        method: Token,
    },
    This {
        token: Token,
    },
    Func {
        params: Vec<Token>,
        body: Vec<Box<Expr>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr {
        expr: Expr,
    },
    Variable {
        name: Token,
        init: Expr,
    },
    If {
        condition: Expr,
        then: Box<Stmt>,
        elif: Option<Box<Stmt>>,
    },
    Block {
        statements: Vec<Box<Stmt>>,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
        token: Token,
    },
    Func {
        token: Token,
        func: Expr,
    },
    Return {
        token: Token,
        value: Expr,
    },
    Break,
    Continue,
    Class {
        name: Token,
        superclass: Expr,
        methods: Vec<Stmt>,
        statics: Vec<Stmt>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    EXPR(Expr),
    STMT(Stmt),
}
