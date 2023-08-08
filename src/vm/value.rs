use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::ops;
use std::sync::Arc;

use super::function::Function;
use crate::vm::gc::heap::{Heap, Object};

use dyn_clone::{clone_trait_object, DynClone};

pub type Value = Box<dyn ValueTrait>;

/// Models a generic value that can be stored in a local variable or on the stack
pub trait ValueTrait: fmt::Display + DynClone {
    fn truthy(&self) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn type_str(&self) -> String;
    fn equal(&self, other: &Value) -> bool;
    fn less_than(&self, other: &Value) -> bool;
    fn greater_than(&self, other: &Value) -> bool;
    fn less_than_or_eq(&self, other: &Value) -> bool;
    fn greater_than_or_eq(&self, other: &Value) -> bool;
}

clone_trait_object!(ValueTrait);

#[macro_export]
macro_rules! as_t {
    ($val: expr, $type: ty) => {
        $val.as_any().downcast_ref::<$type>()
    };
}

#[macro_export]
macro_rules! force_as_t {
    ($val: expr, $type: ty) => {
        $val.as_any().downcast_ref::<$type>().unwrap()
    };
}

impl ops::Add for Value {
    type Output = Result<Value, String>;

    /// Tries to add two values together
    fn add(self, rhs: Self) -> Self::Output {
        if let Some(a) = as_t!(self, FInt) {
            if let Some(b) = as_t!(rhs, FInt) {
                Ok(FInt::new(a.0 + b.0))
            } else if let Some(b) = as_t!(rhs, FFloat) {
                Ok(FFloat::new(a.0 as f64 + b.0))
            } else {
                Err(format!("cannot add int and {}", rhs.type_str()))
            }
        } else if let Some(a) = as_t!(self, FFloat) {
            if let Some(b) = as_t!(rhs, FFloat) {
                Ok(FFloat::new(a.0 + b.0))
            } else if let Some(b) = as_t!(rhs, FInt) {
                Ok(FFloat::new(a.0 + b.0 as f64))
            } else {
                Err(format!("cannot add float and {}", rhs.type_str()))
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

impl ops::Sub for Value {
    type Output = Result<Value, String>;

    /// Tries to subtract `rhs` from `self`
    fn sub(self, rhs: Self) -> Self::Output {
        if let Some(a) = as_t!(self, FInt) {
            if let Some(b) = as_t!(rhs, FInt) {
                Ok(FInt::new(a.0 - b.0))
            } else if let Some(b) = as_t!(rhs, FFloat) {
                Ok(Box::new(FFloat(a.0 as f64 - b.0)))
            } else {
                Err(format!("cannot subtract {} from int", rhs.type_str()))
            }
        } else if let Some(a) = as_t!(self, FFloat) {
            if let Some(b) = as_t!(rhs, FFloat) {
                Ok(FFloat::new(a.0 - b.0))
            } else if let Some(b) = as_t!(rhs, FInt) {
                Ok(FFloat::new(a.0 - b.0 as f64))
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

impl ops::Mul for Value {
    type Output = Result<Value, String>;

    /// Tries to multiply `self` by `rhs`
    fn mul(self, rhs: Self) -> Self::Output {
        if let Some(a) = as_t!(self, FInt) {
            if let Some(b) = as_t!(rhs, FInt) {
                Ok(FInt::new(a.0 * b.0))
            } else if let Some(b) = as_t!(rhs, FFloat) {
                Ok(FFloat::new(a.0 as f64 * b.0))
            } else {
                Err(format!("cannot multiply int by {}", rhs.type_str()))
            }
        } else if let Some(a) = as_t!(self, FFloat) {
            if let Some(b) = as_t!(rhs, FFloat) {
                Ok(FFloat::new(a.0 * b.0))
            } else if let Some(b) = as_t!(rhs, FInt) {
                Ok(FFloat::new(a.0 * b.0 as f64))
            } else {
                Err(format!("cannot multiply float by {}", rhs.type_str()))
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

impl ops::Div for Value {
    type Output = Result<Value, String>;

    /// Tries to divide `self` by `rhs`
    fn div(self, rhs: Self) -> Self::Output {
        if let Some(a) = as_t!(self, FInt) {
            if let Some(b) = as_t!(rhs, FInt) {
                if b.0 == 0 {
                    Err("cannot divide by 0".to_string())
                } else {
                    Ok(FInt::new(a.0 / b.0))
                }
            } else if let Some(b) = as_t!(rhs, FFloat) {
                if b.0 == 0.0 {
                    Err("cannot divide by 0".to_string())
                } else {
                    Ok(FFloat::new(a.0 as f64 / b.0))
                }
            } else {
                Err(format!("cannot divide int by {}", rhs.type_str()))
            }
        } else if let Some(a) = as_t!(self, FFloat) {
            if let Some(b) = as_t!(rhs, FFloat) {
                Ok(FFloat::new(a.0 - b.0))
            } else if let Some(b) = as_t!(rhs, FInt) {
                Ok(FFloat::new(a.0 - b.0 as f64))
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

impl ops::Rem for Value {
    type Output = Result<Value, String>;

    fn rem(self, rhs: Self) -> Self::Output {
        if let Some(a) = as_t!(self, FInt) {
            if let Some(b) = as_t!(rhs, FInt) {
                Ok(FInt::new(a.0 % b.0))
            } else if let Some(b) = as_t!(rhs, FFloat) {
                Ok(FFloat::new(a.0 as f64 % b.0))
            } else {
                Err(format!(
                    "cannot modulus operation with int and {}",
                    rhs.type_str()
                ))
            }
        } else if let Some(a) = as_t!(self, FFloat) {
            if let Some(b) = as_t!(rhs, FFloat) {
                Ok(FFloat::new(a.0 - b.0))
            } else if let Some(b) = as_t!(rhs, FInt) {
                Ok(FFloat::new(a.0 - b.0 as f64))
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

impl ops::Neg for Value {
    type Output = Result<Value, String>;

    fn neg(self) -> Self::Output {
        if let Some(a) = as_t!(self, FInt) {
            Ok(FInt::new(-a.0))
        } else if let Some(a) = as_t!(self, FFloat) {
            Ok(FFloat::new(-a.0))
        } else {
            Err(format!("cannot negate {}", self.type_str()))
        }
    }
}

impl ops::Not for Value {
    type Output = Result<Value, String>;

    fn not(self) -> Self::Output {
        if let Some(a) = as_t!(self, FBool) {
            Ok(FBool::new(!a.0))
        } else {
            Err(format!("cannot negate {} as boolean", self.type_str()))
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

    fn equal(&self, _: &Value) -> bool {
        true
    }

    fn less_than(&self, _: &Value) -> bool {
        true
    }

    fn greater_than(&self, _: &Value) -> bool {
        true
    }

    fn less_than_or_eq(&self, _: &Value) -> bool {
        true
    }

    fn greater_than_or_eq(&self, _: &Value) -> bool {
        true
    }
}
impl fmt::Display for FEmpty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_")
    }
}
impl FEmpty {
    pub fn new() -> Value {
        Box::new(FEmpty)
    }
}

#[derive(Clone)]
pub struct FStr(Object);
impl ValueTrait for FStr {
    fn truthy(&self) -> bool {
        !self.inner().is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "str".to_string()
    }

    fn equal(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FStr) {
            other.inner() == self.inner()
        } else {
            false
        }
    }

    fn less_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FStr) {
            self.inner() < other.inner()
        } else {
            false
        }
    }

    fn greater_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FStr) {
            self.inner() > other.inner()
        } else {
            false
        }
    }

    fn less_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FStr) {
            self.inner() <= other.inner()
        } else {
            false
        }
    }

    fn greater_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FStr) {
            self.inner() >= other.inner()
        } else {
            false
        }
    }
}
impl fmt::Display for FStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.inner())
    }
}
impl FStr {
    pub fn new(heap: &mut Heap, val: String) -> Value {
        Box::new(FStr(heap.allocate(val)))
    }

    pub fn inner_mut(&mut self) -> &mut String {
        unsafe { (self.0.ptr as *mut String).as_mut().unwrap() }
    }

    pub fn inner(&self) -> &String {
        unsafe { (self.0.ptr as *const String).as_ref().unwrap() }
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

    fn equal(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FAtom) {
            other.0 == self.0
        } else {
            false
        }
    }

    fn less_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FAtom) {
            other.0 < self.0
        } else {
            false
        }
    }

    fn greater_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FAtom) {
            other.0 > self.0
        } else {
            false
        }
    }

    fn less_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FAtom) {
            other.0 >= self.0
        } else {
            false
        }
    }

    fn greater_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FAtom) {
            other.0 <= self.0
        } else {
            false
        }
    }
}
impl fmt::Display for FAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!(":{}", self.0))
    }
}
impl FAtom {
    pub fn new(val: Arc<str>) -> Value {
        Box::new(FAtom(val))
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

    fn equal(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FVar) {
            other.0 == self.0
        } else {
            false
        }
    }

    fn less_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FVar) {
            other.0 < self.0
        } else {
            false
        }
    }

    fn greater_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FVar) {
            other.0 > self.0
        } else {
            false
        }
    }

    fn less_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FVar) {
            other.0 >= self.0
        } else {
            false
        }
    }

    fn greater_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FVar) {
            other.0 <= self.0
        } else {
            false
        }
    }
}
impl fmt::Display for FVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("v:{}", self.0))
    }
}
impl FVar {
    pub fn new(val: Arc<str>) -> Value {
        Box::new(FVar(val))
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

    fn equal(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FInt) {
            other.0 == self.0
        } else if let Some(other) = as_t!(other, FFloat) {
            self.0 as f64 == other.0
        } else {
            false
        }
    }

    fn less_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FInt) {
            other.0 < self.0
        } else if let Some(other) = as_t!(other, FFloat) {
            (self.0 as f64) < other.0
        } else {
            false
        }
    }

    fn greater_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FInt) {
            other.0 > self.0
        } else if let Some(other) = as_t!(other, FFloat) {
            (self.0 as f64) > other.0
        } else {
            false
        }
    }

    fn less_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FInt) {
            other.0 <= self.0
        } else if let Some(other) = as_t!(other, FFloat) {
            (self.0 as f64) <= other.0
        } else {
            false
        }
    }

    fn greater_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FInt) {
            other.0 >= self.0
        } else if let Some(other) = as_t!(other, FFloat) {
            (self.0 as f64) >= other.0
        } else {
            false
        }
    }
}
impl fmt::Display for FInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{}", self.0))
    }
}
impl FInt {
    pub fn new(val: i64) -> Value {
        Box::new(FInt(val))
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

    fn equal(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FInt) {
            other.0 as f64 == self.0
        } else if let Some(other) = as_t!(other, FFloat) {
            self.0 == other.0
        } else {
            false
        }
    }

    fn less_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FInt) {
            (other.0 as f64) < self.0
        } else if let Some(other) = as_t!(other, FFloat) {
            self.0 < other.0
        } else {
            false
        }
    }

    fn greater_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FInt) {
            (other.0 as f64) > self.0
        } else if let Some(other) = as_t!(other, FFloat) {
            self.0 > other.0
        } else {
            false
        }
    }

    fn less_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FInt) {
            (other.0 as f64) <= self.0
        } else if let Some(other) = as_t!(other, FFloat) {
            self.0 <= other.0
        } else {
            false
        }
    }

    fn greater_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FInt) {
            (other.0 as f64) >= self.0
        } else if let Some(other) = as_t!(other, FFloat) {
            self.0 >= other.0
        } else {
            false
        }
    }
}
impl fmt::Display for FFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{}", self.0))
    }
}
impl FFloat {
    pub fn new(val: f64) -> Value {
        Box::new(FFloat(val))
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

    fn equal(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FBool) {
            other.0 == self.0
        } else {
            false
        }
    }

    fn less_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FBool) {
            !other.0 & self.0
        } else {
            false
        }
    }

    fn greater_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FBool) {
            other.0 & !self.0
        } else {
            false
        }
    }

    fn less_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FBool) {
            other.0 <= self.0
        } else {
            false
        }
    }

    fn greater_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FBool) {
            other.0 >= self.0
        } else {
            false
        }
    }
}
impl fmt::Display for FBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{}", self.0))
    }
}
impl FBool {
    pub fn new(val: bool) -> Value {
        Box::new(FBool(val))
    }
}

#[derive(Clone)]
pub struct FList(Object);
impl ValueTrait for FList {
    fn truthy(&self) -> bool {
        !self.inner().is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "list".to_string()
    }

    fn equal(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FList) {
            let mut is_equal = true;
            for (a, b) in self.inner().iter().zip(other.inner().iter()) {
                if !a.equal(b) {
                    is_equal = false;
                    break;
                }
            }
            is_equal
        } else {
            false
        }
    }

    fn less_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FList) {
            self.inner().len() < other.inner().len()
        } else {
            false
        }
    }

    fn greater_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FList) {
            self.inner().len() > other.inner().len()
        } else {
            false
        }
    }

    fn less_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FList) {
            self.inner().len() <= other.inner().len()
        } else {
            false
        }
    }

    fn greater_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FList) {
            self.inner().len() >= other.inner().len()
        } else {
            false
        }
    }
}
impl fmt::Display for FList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let list = self
            .inner()
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        f.write_str(&format!("[{}]", list))
    }
}
impl FList {
    pub fn new(heap: &mut Heap, list: Vec<Value>) -> Value {
        Box::new(FList(heap.allocate(list)))
    }

    pub fn inner_mut(&self) -> *mut Vec<Value> {
        self.0.ptr as *mut Vec<Value>
    }

    pub fn inner(&self) -> &Vec<Value> {
        unsafe { (self.0.ptr as *const Vec<Value>).as_ref().unwrap() }
    }
}

#[derive(Clone)]
pub struct FObj(Object);
impl ValueTrait for FObj {
    fn truthy(&self) -> bool {
        !self.inner().is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "obj".to_string()
    }

    fn equal(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FObj) {
            let mut is_equal = true;
            for ((a_key, a_val), (b_key, b_val)) in self.inner().iter().zip(other.inner().iter()) {
                if a_key != b_key && !a_val.equal(b_val) {
                    is_equal = false;
                    break;
                }
            }
            is_equal
        } else {
            false
        }
    }

    fn less_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FObj) {
            self.inner().len() < other.inner().len()
        } else {
            false
        }
    }

    fn greater_than(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FObj) {
            self.inner().len() > other.inner().len()
        } else {
            false
        }
    }

    fn less_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FObj) {
            self.inner().len() <= other.inner().len()
        } else {
            false
        }
    }

    fn greater_than_or_eq(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FObj) {
            self.inner().len() >= other.inner().len()
        } else {
            false
        }
    }
}
impl fmt::Display for FObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let obj = self
            .inner()
            .iter()
            .map(|(k, v)| format!("{}->{}", k.as_ref(), v))
            .collect::<Vec<String>>()
            .join(", ");
        let mut string = "{".to_string();
        string.push_str(&obj);
        string.push('}');
        f.write_str(&string)
    }
}
impl FObj {
    pub fn new(heap: &mut Heap, obj: HashMap<Arc<str>, Value>) -> Value {
        Box::new(FObj(heap.allocate(obj)))
    }

    pub fn inner_mut(&self) -> *mut HashMap<Arc<str>, Value> {
        self.0.ptr as *mut HashMap<Arc<str>, Value>
    }

    pub fn inner(&self) -> &HashMap<Arc<str>, Value> {
        unsafe {
            (self.0.ptr as *const HashMap<Arc<str>, Value>)
                .as_ref()
                .unwrap()
        }
    }
}

#[derive(Clone)]
pub struct FNil;
impl ValueTrait for FNil {
    fn truthy(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "nil".to_string()
    }

    fn equal(&self, other: &Value) -> bool {
        as_t!(other, FEmpty).is_some() || as_t!(other, FNil).is_some()
    }

    fn less_than(&self, _: &Value) -> bool {
        false
    }

    fn greater_than(&self, _: &Value) -> bool {
        false
    }

    fn less_than_or_eq(&self, _: &Value) -> bool {
        false
    }

    fn greater_than_or_eq(&self, _: &Value) -> bool {
        false
    }
}
impl fmt::Display for FNil {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("nil")
    }
}
impl FNil {
    pub fn new() -> Value {
        Box::new(FNil)
    }
}

#[derive(Clone)]
pub struct FFunc(Object);
impl ValueTrait for FFunc {
    fn truthy(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        let func = self.inner();
        let mut ret = format!("fn({}", func.params);
        if func.rest {
            ret += ", +)";
        } else {
            ret += ")";
        }
        ret
    }

    fn equal(&self, other: &Value) -> bool {
        if as_t!(other, FEmpty).is_some() {
            true
        } else if let Some(other) = as_t!(other, FFunc) {
            other.inner().addr == self.inner().addr
                && other.inner().params == self.inner().params
                && other.inner().rest == self.inner().rest
        } else {
            false
        }
    }

    fn less_than(&self, _: &Value) -> bool {
        false
    }

    fn greater_than(&self, _: &Value) -> bool {
        false
    }

    fn less_than_or_eq(&self, _: &Value) -> bool {
        false
    }

    fn greater_than_or_eq(&self, _: &Value) -> bool {
        false
    }
}
impl fmt::Display for FFunc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.type_str())
    }
}
impl FFunc {
    pub fn new(heap: &mut Heap, func: Function) -> Value {
        Box::new(FFunc(heap.allocate(func)))
    }

    pub fn inner_mut(&self) -> *mut Function {
        self.0.ptr as *mut Function
    }

    pub fn inner(&self) -> &Function {
        unsafe { (self.0.ptr as *const Function).as_ref().unwrap() }
    }
}
