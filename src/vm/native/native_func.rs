use std::any::Any;
use std::fmt;

use crate::vm::value::{Value, ValueTrait};
use crate::vm::VM;

pub type NativeFunc = fn(&mut VM, Vec<Value>) -> Value;

#[derive(Clone)]
pub struct FNative(pub NativeFunc);
impl ValueTrait for FNative {
    fn truthy(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_str(&self) -> String {
        "fn<native>".to_string()
    }

    fn equal(&self, _: &Value) -> bool {
        false
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
impl fmt::Display for FNative {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.type_str())
    }
}
impl FNative {
    pub fn new(func: NativeFunc) -> Value {
        Box::new(FNative(func))
    }
}
