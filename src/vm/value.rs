use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Every value in Flan implements this trait
pub trait ValueTrait<T: fmt::Display = Self>: fmt::Display + Clone {
    fn truthy(&self) -> bool;
}

#[derive(Clone)]
pub struct FEmpty;
impl ValueTrait for FEmpty {
    fn truthy(&self) -> bool {
        false
    }
}
impl fmt::Display for FEmpty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_")
    }
}

#[derive(Clone)]
pub struct FStr(String);
impl ValueTrait for FStr {
    fn truthy(&self) -> bool {
        !self.0.is_empty()
    }
}
impl fmt::Display for FStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone)]
pub struct FAtom(Arc<str>);
impl ValueTrait for FAtom {
    fn truthy(&self) -> bool {
        true
    }
}
impl fmt::Display for FAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!(":{}", self.0))
    }
}

#[derive(Clone)]
pub struct FVar(Arc<str>);
impl ValueTrait for FVar {
    fn truthy(&self) -> bool {
        false
    }
}
impl fmt::Display for FVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("v:{}", self.0))
    }
}

#[derive(Clone)]
pub struct FInt(i64);
impl ValueTrait for FInt {
    fn truthy(&self) -> bool {
        self.0 != 0
    }
}
impl fmt::Display for FInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{}", self.0))
    }
}

#[derive(Clone)]
pub struct FFloat(f64);
impl ValueTrait for FFloat {
    fn truthy(&self) -> bool {
        self.0 != 0.0
    }
}
impl fmt::Display for FFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{}", self.0))
    }
}

#[derive(Clone)]
pub struct FBool(bool);
impl ValueTrait for FBool {
    fn truthy(&self) -> bool {
        self.0
    }
}
impl fmt::Display for FBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{}", self.0))
    }
}

#[derive(Clone)]
pub struct FList<T: ValueTrait>(Vec<T>);
impl<T: ValueTrait> ValueTrait for FList<T> {
    fn truthy(&self) -> bool {
        !self.0.is_empty()
    }
}
impl<T: ValueTrait> fmt::Display for FList<T> {
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
pub struct FObj<T: ValueTrait>(HashMap<Arc<str>, T>);
impl<T: ValueTrait> ValueTrait for FObj<T> {
    fn truthy(&self) -> bool {
        !self.0.is_empty()
    }
}
impl<T: ValueTrait> fmt::Display for FObj<T> {
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
