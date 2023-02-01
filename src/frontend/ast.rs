use super::token::Token;
use crate::bulk_print;

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
    FloatLiteral {
        token: Token,
        value: f64,
    },
    IntegerLiteral {
        token: Token,
        value: i64,
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
        token: Token,
        values: Vec<Box<Expr>>,
    },
    ObjectLiteral {
        token: Token,
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
        init: bool,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Box<Expr>>,
        token: Token,
    },
    Get {
        instance: Box<Expr>,
        value: Box<Expr>,
        token: Token,
    },
    Set {
        instance: Box<Expr>,
        token: Token,
        value: Box<Expr>,
    },
    Func {
        name: Option<Token>,
        params: Vec<Token>,
        body: Box<Expr>,
    },
    Match {
        token: Token,
        condition: Box<Expr>,
        branches: Vec<MatchBranch>,
    },
    Block {
        exprs: Vec<Box<Expr>>,
    },
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchBranch {
    pub target: Box<Expr>,
    pub body: Box<Expr>,
}

impl Expr {
    pub fn pretty_print(exprs: &Vec<Expr>) -> String {
        exprs
            .into_iter()
            .map(|expr| expr.print())
            .collect::<Vec<String>>()
            .join("\n")
    }

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
            Expr::IntegerLiteral { token: _, value } => {
                format!("{}", value)
            }
            Expr::FloatLiteral { token: _, value } => {
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
            Expr::ListLiteral { token: _, values } => {
                if values.len() > 0 {
                    format!("(list {})", bulk_print!(values, " "))
                } else {
                    String::from("(list)")
                }
            }
            Expr::ObjectLiteral {
                token: _,
                keys,
                values,
            } => {
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
            Expr::Assign { left, right, init } => {
                format!(
                    "(assign{} {} {})",
                    if *init { "I" } else { "" },
                    left.print(),
                    right.print()
                )
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
            Expr::Get {
                instance,
                value,
                token: _,
            } => {
                format!("{}.{}", instance.print(), value.print())
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
            Expr::Func { name, params, body } => {
                if let Some(name) = name {
                    format!(
                        "(func {} ({}) {})",
                        name.print(),
                        bulk_print!(params, " "),
                        body.print()
                    )
                } else {
                    format!("(lambda ({}) {})", bulk_print!(params, " "), body.print(),)
                }
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
            Expr::Block { exprs } => {
                format!("(block{})", {
                    let expr = bulk_print!(exprs, " ");
                    if expr == "" {
                        String::new()
                    } else {
                        String::from(" ") + &expr
                    }
                })
            }
            Expr::End => "".to_string(),
        }
    }
}
