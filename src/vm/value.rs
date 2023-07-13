use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::ops;
use std::sync::Arc;

use dyn_clone::{clone_trait_object, DynClone};

/// Every value in Flan implements this trait
pub trait ValueTrait: fmt::Display + DynClone {
    fn truthy(&self) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn type_str(&self) -> String;
}

clone_trait_object!(ValueTrait);

macro_rules! as_t {
    ($val: expr, $type: ty) => {
        $val.as_any().downcast_ref::<$type>()
    };
}

macro_rules! force_as_t {
    ($val: expr, $type: ty) => {
        as_t($val, $type).unwrap()
    };
}

impl ops::Add for Box<dyn ValueTrait> {
    type Output = Result<Box<dyn ValueTrait>, String>;

    /// Tries to add two values together
    fn add(self, rhs: Self) -> Self::Output {
        if let Some(a) = as_t!(self, FInt) {
            if let Some(b) = as_t!(rhs, FInt) {
                Ok(Box::new(FInt(a.0 + b.0)))
            } else if let Some(b) = as_t!(rhs, FFloat) {
                Ok(Box::new(FFloat(a.0 as f64 + b.0)))
            } else {
                Err(format!("cannot add int and {}", rhs.type_str()))
            }
        } else if let Some(a) = as_t!(self, FFloat) {
            if let Some(b) = as_t!(rhs, FFloat) {
                Ok(Box::new(FFloat(a.0 + b.0)))
            } else if let Some(b) = as_t!(rhs, FInt) {
                Ok(Box::new(FFloat(a.0 + b.0 as f64)))
            } else {
                Err(format!("cannot add float and {}", rhs.type_str()))
            }
        } else if let Some(a) = as_t!(self, FStr) {
            if let Some(b) = as_t!(rhs, FStr) {
                Ok(Box::new(FStr(a.0.clone() + b.0.as_str())))
            } else {
                Err(format!("cannot add str and {}", rhs.type_str()))
            }
        } else {
            Err(format!(
                "cannot add {} and {}",
                self.type_str(),
                rhs.type_str()
            ))
        }
    }
}

impl ops::Sub for Box<dyn ValueTrait> {
    type Output = Result<Box<dyn ValueTrait>, String>;

    /// Tries to subtract `rhs` from `self`
    fn sub(self, rhs: Self) -> Self::Output {
        if let Some(a) = as_t!(self, FInt) {
            if let Some(b) = as_t!(rhs, FInt) {
                Ok(Box::new(FInt(a.0 - b.0)))
            } else if let Some(b) = as_t!(rhs, FFloat) {
                Ok(Box::new(FFloat(a.0 as f64 - b.0)))
            } else {
                Err(format!("cannot subtract {} from int", rhs.type_str()))
            }
        } else if let Some(a) = as_t!(self, FFloat) {
            if let Some(b) = as_t!(rhs, FFloat) {
                Ok(Box::new(FFloat(a.0 - b.0)))
            } else if let Some(b) = as_t!(rhs, FInt) {
                Ok(Box::new(FFloat(a.0 - b.0 as f64)))
            } else {
                Err(format!("cannot subtract {} from float", rhs.type_str()))
            }
        } else {
            Err(format!(
                "cannot subtract {} from {}",
                rhs.type_str(),
                self.type_str(),
            ))
        }
    }
}

impl ops::Mul for Box<dyn ValueTrait> {
    type Output = Result<Box<dyn ValueTrait>, String>;

    /// Tries to multiply `self` by `rhs`
    fn mul(self, rhs: Self) -> Self::Output {
        if let Some(a) = as_t!(self, FInt) {
            if let Some(b) = as_t!(rhs, FInt) {
                Ok(Box::new(FInt(a.0 * b.0)))
            } else if let Some(b) = as_t!(rhs, FFloat) {
                Ok(Box::new(FFloat(a.0 as f64 * b.0)))
            } else {
                Err(format!("cannot multiply int by {}", rhs.type_str()))
            }
        } else if let Some(a) = as_t!(self, FFloat) {
            if let Some(b) = as_t!(rhs, FFloat) {
                Ok(Box::new(FFloat(a.0 * b.0)))
            } else if let Some(b) = as_t!(rhs, FInt) {
                Ok(Box::new(FFloat(a.0 * b.0 as f64)))
            } else {
                Err(format!("cannot multiply float by {}", rhs.type_str()))
            }
        } else if let Some(a) = as_t!(self, FStr) {
            if let Some(b) = as_t!(rhs, FInt) {
                let mut val = String::new();
                for _ in 0..b.0 {
                    val.push_str(a.0.clone().as_str());
                }
                Ok(Box::new(FStr(val)))
            } else {
                Err(format!("cannot multiply str by {}", rhs.type_str()))
            }
        } else {
            Err(format!(
                "cannot multiply {} by {}",
                self.type_str(),
                rhs.type_str(),
            ))
        }
    }
}

impl ops::Div for Box<dyn ValueTrait> {
    type Output = Result<Box<dyn ValueTrait>, String>;

    /// Tries to divide `self` by `rhs`
    fn div(self, rhs: Self) -> Self::Output {
        if let Some(a) = as_t!(self, FInt) {
            if let Some(b) = as_t!(rhs, FInt) {
                if b.0 == 0 {
                    Err("cannot divide by 0".to_string())
                } else {
                    Ok(Box::new(FInt(a.0 / b.0)))
                }
            } else if let Some(b) = as_t!(rhs, FFloat) {
                if b.0 == 0.0 {
                    Err("cannot divide by 0".to_string())
                } else {
                    Ok(Box::new(FFloat(a.0 as f64 / b.0)))
                }
            } else {
                Err(format!("cannot divide int by {}", rhs.type_str()))
            }
        } else if let Some(a) = as_t!(self, FFloat) {
            if let Some(b) = as_t!(rhs, FFloat) {
                Ok(Box::new(FFloat(a.0 - b.0)))
            } else if let Some(b) = as_t!(rhs, FInt) {
                Ok(Box::new(FFloat(a.0 - b.0 as f64)))
            } else {
                Err(format!("cannot divide float by {}", rhs.type_str()))
            }
        } else {
            Err(format!(
                "cannot divide {} by {}",
                self.type_str(),
                rhs.type_str(),
            ))
        }
    }
}

impl ops::Rem for Box<dyn ValueTrait> {
    type Output = Result<Box<dyn ValueTrait>, String>;

    fn rem(self, rhs: Self) -> Self::Output {
        if let Some(a) = as_t!(self, FInt) {
            if let Some(b) = as_t!(rhs, FInt) {
                Ok(Box::new(FInt(a.0 % b.0)))
            } else if let Some(b) = as_t!(rhs, FFloat) {
                Ok(Box::new(FFloat(a.0 as f64 % b.0)))
            } else {
                Err(format!(
                    "cannot modulus operation with int and {}",
                    rhs.type_str()
                ))
            }
        } else if let Some(a) = as_t!(self, FFloat) {
            if let Some(b) = as_t!(rhs, FFloat) {
                Ok(Box::new(FFloat(a.0 - b.0)))
            } else if let Some(b) = as_t!(rhs, FInt) {
                Ok(Box::new(FFloat(a.0 - b.0 as f64)))
            } else {
                Err(format!(
                    "cannot modulus operation with float by {}",
                    rhs.type_str()
                ))
            }
        } else {
            Err(format!(
                "cannot modulus operation with {} by {}",
                self.type_str(),
                rhs.type_str(),
            ))
        }
    }
}

#[derive(Clone)]
pub struct FEmpty;
impl ValueTrait for FEmpty {
    fn truthy(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "_".to_string()
    }
}
impl fmt::Display for FEmpty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_")
    }
}

#[derive(Clone)]
pub struct FStr(pub String);
impl ValueTrait for FStr {
    fn truthy(&self) -> bool {
        !self.0.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "str".to_string()
    }
}
impl fmt::Display for FStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone)]
pub struct FAtom(pub Arc<str>);
impl ValueTrait for FAtom {
    fn truthy(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "atom".to_string()
    }
}
impl fmt::Display for FAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!(":{}", self.0))
    }
}

#[derive(Clone)]
pub struct FVar(pub Arc<str>);
impl ValueTrait for FVar {
    fn truthy(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "var".to_string()
    }
}
impl fmt::Display for FVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("v:{}", self.0))
    }
}

#[derive(Clone)]
pub struct FInt(pub i64);
impl ValueTrait for FInt {
    fn truthy(&self) -> bool {
        self.0 != 0
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "int".to_string()
    }
}
impl fmt::Display for FInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{}", self.0))
    }
}

#[derive(Clone)]
pub struct FFloat(pub f64);
impl ValueTrait for FFloat {
    fn truthy(&self) -> bool {
        self.0 != 0.0
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "float".to_string()
    }
}
impl fmt::Display for FFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{}", self.0))
    }
}

#[derive(Clone)]
pub struct FBool(pub bool);
impl ValueTrait for FBool {
    fn truthy(&self) -> bool {
        self.0
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "bool".to_string()
    }
}
impl fmt::Display for FBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{}", self.0))
    }
}

#[derive(Clone)]
pub struct FList(pub Vec<Box<dyn ValueTrait>>);
impl ValueTrait for FList {
    fn truthy(&self) -> bool {
        !self.0.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "list".to_string()
    }
}
impl fmt::Display for FList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let list = self
            .0
            .clone()
            .into_iter()
            .map(|v| v.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        f.write_str(&format!("[{}]", list))
    }
}

#[derive(Clone)]
pub struct FObj(pub HashMap<Arc<str>, Box<dyn ValueTrait>>);
impl ValueTrait for FObj {
    fn truthy(&self) -> bool {
        !self.0.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "obj".to_string()
    }
}
impl fmt::Display for FObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut obj = self
            .0
            .clone()
            .into_iter()
            .map(|(k, v)| format!("{}->{}", k, v))
            .collect::<Vec<String>>()
            .join(", ");
        obj.push('}');
        let mut string = "{".to_string();
        string.push_str(&obj);
        string.push('}');
        f.write_str(&obj)
    }
}
