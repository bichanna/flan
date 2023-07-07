use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use dyn_clone::{clone_trait_object, DynClone};

/// Every value in Flan implements this trait
pub trait ValueTrait: fmt::Display + DynClone {
    fn truthy(&self) -> bool;
    fn as_any(&self) -> &dyn Any;
}

clone_trait_object!(ValueTrait);

#[derive(Clone)]
pub struct FEmpty;
impl ValueTrait for FEmpty {
    fn truthy(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
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
