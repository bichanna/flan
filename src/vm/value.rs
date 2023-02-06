use super::object::{Object, ObjectType, ObjectUnion};

#[derive(Clone, Copy)]
pub enum Value {
    Null,
    Empty,
    Bool(bool),
    Int(i64),
    Float(f64),
    Object(Object),
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
            Value::Object(obj) => match obj.obj_type {
                ObjectType::String => "string".to_string(),
                ObjectType::Atom => "atom".to_string(),
                ObjectType::List => "list".to_string(),
                ObjectType::Object => "object".to_string(),
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
            Self::Object(l) => match l.obj_type {
                ObjectType::String => match rhs {
                    Self::Object(r) => match r.obj_type {
                        ObjectType::String => {
                            // Concatenate two strings
                            let mut new_str = unsafe { (*(*l.obj).string).clone() }
                                + (&(unsafe { (*(*r.obj).string).clone() }));

                            let obj = Object {
                                obj_type: ObjectType::String,
                                obj: &mut ObjectUnion {
                                    string: &mut new_str as *mut String,
                                } as *mut ObjectUnion,
                            };

                            Ok(Self::Object(obj))
                        }
                        _ => Err(format!("cannot add string and {}", rhs.type_())),
                    },
                    _ => Err(format!("cannot add string and {}", rhs.type_())),
                },
                _ => Err(format!("cannot add string and {}", rhs.type_())),
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
            _ => Err(format!("cannot negate {}", self.type_())),
        }
    }
}

/// Not (boolean negation)
impl std::ops::Not for Value {
    type Output = Result<Value, String>;

    fn not(self) -> Self::Output {
        match self {
            Self::Bool(l) => Ok(Self::Bool(!l)),
            _ => Err(format!("cannot negate {} as Boolean", self.type_())),
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
