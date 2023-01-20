use super::scope::Scope;
use crate::ast::Expr;

#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Empty,
    Null,
    Str(String),
    Num(f64),
    Bool(bool),
    Atom(String),
    List(Vec<Box<Value<'a>>>),
    Object((Vec<String>, Vec<Box<Value<'a>>>)),
    Func(Expr, Scope<'a>),
    Thunk(Expr, Scope<'a>),
}

impl<'a> Value<'a> {
    pub fn equal(&self, other: &Value) -> bool {
        match *self {
            Value::Empty => true,
            Value::Thunk(_, _) => false,
            _ => match *other {
                Value::Empty => true,
                Value::Thunk(_, _) => false,
                _ => self == other,
            },
        }
    }

    pub fn string(&self, inside: bool) -> String {
        match self {
            Value::Empty => String::from("_"),
            Value::Null => String::from("null"),
            Value::Str(v) => {
                if inside {
                    format!("\"{}\"", v)
                } else {
                    v.clone()
                }
            }
            Value::Bool(v) => format!("{}", v),
            Value::Num(v) => format!("{}", v),
            Value::Atom(v) => String::from(":") + &v,
            Value::List(l) => format!(
                "[{}]",
                l.into_iter()
                    .map(|i| i.string(true))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Value::Object(m) => format!(
                "{}",
                m.0.to_owned()
                    .into_iter()
                    .zip(m.1.to_owned().into_iter())
                    .map(|(k, v)| format!("{}: {}", k, v.string(true)))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Value::Func(fnc, _) => fnc.print(),
            Value::Thunk(fnc, _) => format!("thunk {}", fnc.print()),
        }
    }
}
