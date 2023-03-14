use super::object::RawObject;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    Null,
    Empty,
    Bool(bool),
    Int(i64),
    Float(f64),
    Object(RawObject),
}

impl Value {
    pub fn print(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Empty => "_".to_string(),
            Value::Bool(v) => format!("{}", v),
            Value::Int(v) => format!("{}", v),
            Value::Float(v) => format!("{}", v),
            Value::Object(obj) => obj.print(),
        }
    }

    /// Returns the type of Value as a String, useful for error messages
    pub fn type_(&self) -> String {
        match self {
            Value::Null | Value::Empty => self.print(),
            Value::Bool(_) => "bool".to_string(),
            Value::Int(_) => "integer".to_string(),
            Value::Float(_) => "float".to_string(),
            Value::Object(obj) => match obj {
                RawObject::String(_) => "string".to_string(),
                RawObject::Atom(_) => "atom".to_string(),
                RawObject::List(_) => "list".to_string(),
                RawObject::Object(_) => "object".to_string(),
            },
        }
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
            Self::Object(left) => match left {
                RawObject::String(l) => match rhs {
                    Self::Object(right) => match right {
                        RawObject::String(r) => {
                            let left = unsafe { l.read() };
                            let right = unsafe { r.read() };
                            Ok(Value::Object(RawObject::String(
                                &mut (left + &right) as *mut String,
                            )))
                        }
                        _ => Err(format!("cannot add string and {}", rhs.type_())),
                    },
                    _ => Err(format!("cannot add {} and {}", self.type_(), rhs.type_())),
                },
                _ => Err(format!("cannot add {} and {}", self.type_(), rhs.type_())),
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
