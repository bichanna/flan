use crate::bulk_print;
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
        body: Vec<Node>,
    },
    Unknown,
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
        then: Box<Node>,
        els: Option<Box<Node>>,
    },
    Block {
        statements: Vec<Node>,
    },
    While {
        condition: Expr,
        body: Box<Node>,
        token: Token,
    },
    Func {
        token: Token,
        func: Expr,
    },
    Return {
        token: Token,
        values: Vec<Expr>,
    },
    Break,
    Continue,
    Import {
        name: Expr,
        token: Token,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    EXPR(Expr),
    STMT(Stmt),
}

impl Node {
    pub fn pretty_print(nodes: &Vec<Node>) -> String {
        bulk_print!(nodes, "\n")
    }

    fn print(&self) -> String {
        match self {
            Node::EXPR(expr) => expr.print(),
            Node::STMT(stmt) => stmt.print(),
        }
    }
}

impl Expr {
    pub fn print(&self) -> String {
        match self {
            Expr::Binary { left, right, op } => {
                format!("({} {} {})", op.print(), left.print(), right.print())
            }
            Expr::Group { expr } => {
                format!("({})", expr.print())
            }
            Expr::Unary { right, op } => {
                format!("({} {})", op.print(), right.print())
            }
            Expr::Literal { kind, value } => match kind {
                TokenType::Str => format!("\"{}\"", value),
                TokenType::Num | TokenType::False | TokenType::True | TokenType::Null => {
                    format!("{:?}", kind)
                }
                _ => panic!("invalidddddddd"),
            },
            Expr::Logical { left, right, op } => {
                format!("({} {} {})", op.print(), left.print(), right.print())
            }
            Expr::Variable { name } => {
                format!("{}", name.print())
            }
            Expr::Assign { name, value } => {
                format!("(assign {} {})", name.print(), value.print())
            }
            Expr::Call {
                callee,
                args,
                token: _,
            } => {
                let mut builder = format!("({}", callee.print());
                if args.len() > 0 {
                    builder += &format!(" {})", bulk_print!(args, " "));
                } else {
                    builder += ")";
                }
                builder
            }
            Expr::Get { instance, token } => {
                format!("{}.{}", instance.print(), token.print())
            }
            Expr::Set {
                instance,
                token,
                value,
            } => {
                format!(
                    "(set {}.{} {})",
                    instance.print(),
                    token.print(),
                    value.print()
                )
            }
            Expr::Super { token: _, method } => {
                format!("super.{}", method.print())
            }
            Expr::This { token: _ } => String::from("this"),
            Expr::Func { params, body } => {
                format!(
                    "(lambda ({}) {})",
                    bulk_print!(params, " "),
                    bulk_print!(body, " "),
                )
            }
            Expr::Unknown => String::from("unknown"),
        }
    }
}

impl Stmt {
    fn print(&self) -> String {
        match self {
            Stmt::Expr { expr } => String::from(expr.print()),
            Stmt::Variable { name, init } => {
                format!("(var {} {})", name.print(), init.print())
            }
            Stmt::If {
                condition,
                then,
                els,
            } => {
                let mut builder = format!("(if ({}) {}", condition.print(), then.print());
                if let Some(els) = els {
                    builder += els.print().as_str();
                }
                builder += ")";
                builder
            }
            Stmt::Block { statements } => {
                format!("(block{})", {
                    let stmts = bulk_print!(statements, " ");
                    if stmts == "" {
                        String::new()
                    } else {
                        String::from(" ") + &stmts
                    }
                })
            }
            Stmt::While {
                condition,
                body,
                token: _,
            } => {
                format!("(while ({}) {})", condition.print(), body.print())
            }
            Stmt::Func { token, func } => {
                format!("(func {} {})", token.print(), func.print())
            }
            Stmt::Return { token: _, values } => {
                format!("(return {})", bulk_print!(values, " "))
            }
            Stmt::Break => String::from("(break)"),
            Stmt::Continue => String::from("(continue)"),
            Stmt::Import { name, token: _ } => {
                format!("(import {})", name.print())
            }
        }
    }
}
