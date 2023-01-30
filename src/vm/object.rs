use std::collections::HashMap;

use super::value::Value;

#[derive(Copy, Clone)]
pub struct Object {
    pub obj_type: ObjectType,
    obj: ObjectUnion,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union ObjectUnion {
    string: *mut String,
    object: *mut HashMap<String, Box<Value>>,
    list: *mut Vec<Box<Value>>,
}

#[derive(Copy, Clone)]
pub enum ObjectType {
    List,
    String,
    Atom,
    Object,
}

impl Object {
    /// Frees the object pointed based on its type
    pub fn free(&self) {
        match self.obj_type {
            ObjectType::String | ObjectType::Atom => unsafe { drop(self.obj.string) },
            ObjectType::List => unsafe { drop(self.obj.list) },
            ObjectType::Object => unsafe { drop(self.obj.object) },
        }
    }

    pub fn print(&self) -> String {
        match self.obj_type {
            ObjectType::String => unsafe { (*self.obj.string).to_owned() },
            ObjectType::Atom => {
                let name = unsafe { (*self.obj.string).to_owned() };
                format!(":{}", name)
            }
            ObjectType::List => {
                let list = unsafe { (*self.obj.list).to_owned() };
                format!(
                    "[{}]",
                    list.into_iter()
                        .map(|i| i.print())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            ObjectType::Object => {
                let obj = unsafe { (*self.obj.object).to_owned() };
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
