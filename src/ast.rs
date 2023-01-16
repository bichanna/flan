use crate::bulk_print;
use crate::token::Token;

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
    StringLiteral {
        token: Token,
        value: String,
    },
    NumberLiteral {
        token: Token,
        value: f64,
    },
    BoolLiteral {
        token: Token,
        payload: bool,
    },
    AtomLiteral {
        token: Token,
        value: String,
    },
    Underscore {
        token: Token,
    },
    Null {
        token: Token,
    },
    ListLiteral {
        values: Vec<Box<Expr>>,
    },
    ObjectLiteral {
        keys: Vec<Token>,
        values: Vec<Box<Expr>>,
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
    Access {
        token: Token,
        expr: Box<Expr>,
        index: Box<Expr>,
    },
    Func {
        params: Vec<Token>,
        body: Vec<Node>,
    },
    Import {
        name: Box<Expr>,
        token: Token,
    },
    Match {
        token: Token,
        condition: Box<Expr>,
        branches: Vec<MatchBranch>,
    },
    Block {
        nodes: Vec<Node>,
    },
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchBranch {
    pub target: Box<Expr>,
    pub body: Node,
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
        value: Expr,
    },
    Break,
    Continue,
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
            Expr::StringLiteral { token: _, value } => {
                format!("\"{}\"", value)
            }
            Expr::NumberLiteral { token: _, value } => {
                format!("{}", value)
            }
            Expr::BoolLiteral { token: _, payload } => {
                format!("{}", payload)
            }
            Expr::AtomLiteral { token: _, value } => {
                format!(":{}", value)
            }
            Expr::Underscore { token: _ } => String::from(":_:"),
            Expr::Null { token: _ } => String::from("null"),
            Expr::ListLiteral { values } => {
                if values.len() > 0 {
                    format!("(list {})", bulk_print!(values, " "))
                } else {
                    String::from("(list)")
                }
            }
            Expr::ObjectLiteral { keys, values } => {
                if keys.len() > 0 {
                    format!(
                        "(object {})",
                        keys.into_iter()
                            .zip(values.into_iter())
                            .map(|(k, v)| format!("{}:{}", k.print(), v.print()))
                            .collect::<Vec<String>>()
                            .join(" ")
                    )
                } else {
                    String::from("(object)")
                }
            }
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
            Expr::Access {
                token: _,
                expr,
                index,
            } => {
                format!("(.access {} {})", expr.print(), index.print())
            }
            Expr::Func { params, body } => {
                format!(
                    "(lambda ({}) {})",
                    bulk_print!(params, " "),
                    bulk_print!(body, " "),
                )
            }
            Expr::Import { name, token: _ } => {
                format!("(import {})", name.print())
            }
            Expr::Match {
                token: _,
                condition,
                branches,
            } => {
                let mut builder = format!("(match {}", condition.print());
                if branches.len() > 0 {
                    builder += " ";
                    builder += &branches
                        .into_iter()
                        .map(|x| format!("{} -> {}", x.target.print(), x.body.print()))
                        .collect::<Vec<String>>()
                        .join(" ");
                    builder += ")";
                } else {
                    builder += ")";
                }
                builder
            }
            Expr::Block { nodes } => {
                format!("(block{})", {
                    let stmts = bulk_print!(nodes, " ");
                    if stmts == "" {
                        String::new()
                    } else {
                        String::from(" ") + &stmts
                    }
                })
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
            Stmt::Return { token: _, value } => {
                format!("(return {})", value.print())
            }
            Stmt::Break => String::from("(break)"),
            Stmt::Continue => String::from("(continue)"),
        }
    }
}
