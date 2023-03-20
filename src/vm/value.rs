use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::function::Function;

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Null,
    Empty,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Rc<RefCell<String>>),
    Atom(Rc<String>),
    Var(Rc<String>),
    Object(Rc<RefCell<HashMap<String, Box<Value>>>>),
    List(Rc<RefCell<Vec<Box<Value>>>>),
    Function(Rc<Function>),
}

impl Value {
    pub fn print(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Empty => "_".to_string(),
            Value::Bool(v) => format!("{}", v),
            Value::Int(v) => format!("{}", v),
            Value::Float(v) => format!("{}", v),
            Value::String(v) => format!("{}", v.borrow()),
            Value::Var(v) => format!("var:{}", v),
            Value::Atom(v) => format!(":{}", v),
            Value::List(list) => {
                let list = list.borrow();
                format!(
                    "[{}]",
                    list.clone()
                        .into_iter()
                        .map(|i| i.print())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            Value::Object(obj) => {
                let obj = obj.borrow();
                format!(
                    "{{\n{}\n}}",
                    obj.clone()
                        .into_iter()
                        .map(|(k, v)| format!("\t{}: {}", k, v.print()))
                        .collect::<Vec<String>>()
                        .join(",\n")
                )
            }
            Value::Function(func) => format!("func:{}", func.name),
        }
    }

    /// Returns the type of Value as a String, useful for error messages
    pub fn type_(&self) -> String {
        match self {
            Value::Null | Value::Empty => self.print(),
            Value::Bool(_) => "bool".to_string(),
            Value::Int(_) => "integer".to_string(),
            Value::Float(_) => "float".to_string(),
            Value::String(_) => "string".to_string(),
            Value::Atom(_) => "atom".to_string(),
            Value::Var(_) => "variable".to_string(),
            Value::List(_) => "list".to_string(),
            Value::Object(_) => "object".to_string(),
            Value::Function(_) => "function".to_string(),
        }
    }

    pub fn new_var(name: String) -> Self {
        Self::Var(Rc::new(name))
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(Rc::new(RefCell::new(value)))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Atom(Rc::new(value.to_string()))
    }
}

impl From<Vec<Box<Value>>> for Value {
    fn from(value: Vec<Box<Value>>) -> Self {
        Self::List(Rc::new(RefCell::new(value)))
    }
}

impl From<HashMap<String, Box<Value>>> for Value {
    fn from(value: HashMap<String, Box<Value>>) -> Self {
        Self::Object(Rc::new(RefCell::new(value)))
    }
}

/// Addition
impl std::ops::Add<Value> for Value {
    type Output = Result<Value, String>;

    fn add(self, rhs: Value) -> Self::Output {
        match self {
            Self::Int(l) => match rhs {
                Self::Int(r) => Ok(Self::Int(l + r)),
                Self::Float(r) => Ok(Self::Float(l as f64 + r)),
                _ => Err(format!("cannot add integer and {}", rhs.type_())),
            },
            Self::Float(l) => match rhs {
                Self::Int(r) => Ok(Self::Float(l + r as f64)),
                Self::Float(r) => Ok(Self::Float(l + r)),
                _ => Err(format!("cannot add float and {}", rhs.type_())),
            },
            Self::String(l) => match rhs {
                Self::String(r) => Ok(Self::String(Rc::new(RefCell::new(
                    l.borrow().to_string() + &r.borrow(),
                )))),
                _ => Err(format!("cannot string and {}", rhs.type_())),
            },
            _ => Err(format!("cannot add {} and {}", self.type_(), rhs.type_())),
        }
    }
}

/// Sub
impl std::ops::Sub<Value> for Value {
    type Output = Result<Value, String>;

    fn sub(self, rhs: Value) -> Self::Output {
        match self {
            Self::Int(l) => match rhs {
                Self::Int(r) => Ok(Self::Int(l - r)),
                Self::Float(r) => Ok(Self::Float(l as f64 - r)),
                _ => Err(format!("cannot subtract {} from integer", rhs.type_())),
            },
            Self::Float(l) => match rhs {
                Self::Int(r) => Ok(Self::Float(l - r as f64)),
                Self::Float(r) => Ok(Self::Float(l - r)),
                _ => Err(format!("cannot subtract {} from float", rhs.type_())),
            },
            _ => Err(format!(
                "cannot subtract {} from {}",
                self.type_(),
                rhs.type_(),
            )),
        }
    }
}

/// Mult
impl std::ops::Mul<Value> for Value {
    type Output = Result<Value, String>;

    fn mul(self, rhs: Value) -> Self::Output {
        match self {
            Self::Int(l) => match rhs {
                Self::Int(r) => Ok(Self::Int(l * r)),
                Self::Float(r) => Ok(Self::Float(l as f64 * r)),
                _ => Err(format!("cannot multiply integer by {}", rhs.type_())),
            },
            Self::Float(l) => match rhs {
                Self::Int(r) => Ok(Self::Float(l * r as f64)),
                Self::Float(r) => Ok(Self::Float(l * r)),
                _ => Err(format!("cannot subtract float by {}", rhs.type_())),
            },
            _ => Err(format!(
                "cannot multiply {} by {}",
                self.type_(),
                rhs.type_(),
            )),
        }
    }
}

/// Div
impl std::ops::Div<Value> for Value {
    type Output = Result<Value, String>;

    fn div(self, rhs: Value) -> Self::Output {
        match self {
            Self::Int(l) => match rhs {
                Self::Int(r) => Ok(Self::Int(l / r)),
                Self::Float(r) => Ok(Self::Float(l as f64 / r)),
                _ => Err(format!("cannot divide integer by {}", rhs.type_())),
            },
            Self::Float(l) => match rhs {
                Self::Int(r) => Ok(Self::Float(l / r as f64)),
                Self::Float(r) => Ok(Self::Float(l / r)),
                _ => Err(format!("cannot divide float by {}", rhs.type_())),
            },
            _ => Err(format!("cannot divide {} by {}", self.type_(), rhs.type_(),)),
        }
    }
}

/// Negate
impl std::ops::Neg for Value {
    type Output = Result<Value, String>;

    fn neg(self) -> Self::Output {
        match self {
            Self::Int(l) => Ok(Self::Int(-l)),
            Self::Float(l) => Ok(Self::Float(-l)),
            Self::Bool(l) => Ok(Self::Bool(!l)),
            _ => Err(format!("cannot negate {}", self.type_())),
        }
    }
}

/// Mod
impl std::ops::Rem for Value {
    type Output = Result<Value, String>;

    fn rem(self, rhs: Value) -> Self::Output {
        match self {
            Self::Int(l) => match rhs {
                Self::Int(r) => Ok(Self::Int(l % r)),
                Self::Float(r) => Ok(Self::Float(l as f64 % r)),
                _ => Err(format!("cannot {} % by {}", self.type_(), rhs.type_())),
            },
            Self::Float(l) => match rhs {
                Self::Int(r) => Ok(Self::Float(l % r as f64)),
                Self::Float(r) => Ok(Self::Float(l % r)),
                _ => Err(format!("cannot {} % by {}", self.type_(), rhs.type_())),
            },
            _ => Err(format!("cannot {} % by {}", self.type_(), rhs.type_())),
        }
    }
}
