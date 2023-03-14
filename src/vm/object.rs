use std::collections::HashMap;

use super::value::Value;

#[derive(Debug, Copy, PartialEq, Clone)]
pub enum RawObject {
    String(*mut String),
    Atom(*const String),
    Object(*mut HashMap<String, Box<Value>>),
    List(*mut Vec<Box<Value>>),
}

impl RawObject {
    /// Frees the object pointed
    pub fn free(&self) {
        match self {
            Self::String(v) => drop(v),
            Self::Atom(v) => drop(v),
            Self::Object(obj) => drop(obj),
            Self::List(list) => drop(list),
        }
    }

    pub fn print(&self) -> String {
        match self {
            Self::String(v) => unsafe { v.read() },
            Self::Atom(v) => format!(":{}", unsafe { v.read() }),
            Self::List(list) => {
                let list = unsafe { list.read() };
                format!(
                    "[{}]",
                    list.into_iter()
                        .map(|i| i.print())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            Self::Object(obj) => {
                let obj = unsafe { obj.read() };

                format!(
                    "{{\n{}\n}}",
                    obj.into_iter()
                        .map(|(k, v)| format!("\t{}: {}", k, v.print()))
                        .collect::<Vec<String>>()
                        .join(",\n")
                )
            }
        }
    }
}
