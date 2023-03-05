use std::collections::HashMap;
use std::mem::ManuallyDrop;

use super::value::Value;

#[derive(Debug, Copy, PartialEq, Clone)]
pub enum RawObject {
    String(*mut ManuallyDrop<String>),
    Atom(*const ManuallyDrop<String>),
    Object(*mut ManuallyDrop<HashMap<String, Box<Value>>>),
    List(*mut ManuallyDrop<Vec<Box<Value>>>),
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
            Self::String(v) => ManuallyDrop::into_inner(unsafe { (**v).clone() }),
            Self::Atom(v) => format!(":{}", ManuallyDrop::into_inner(unsafe { (**v).clone() })),
            Self::List(list) => {
                let list = ManuallyDrop::into_inner(unsafe { (**list).clone() });
                format!(
                    "[{}]",
                    list.into_iter()
                        .map(|i| i.print())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            Self::Object(obj) => {
                let obj = ManuallyDrop::into_inner(unsafe { (**obj).clone() });

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
