use std::collections::HashMap;

use super::value::Value;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Object {
    pub obj_type: ObjectType,
    pub obj: *mut ObjectUnion,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union ObjectUnion {
    pub string: *mut String,
    pub object: *mut HashMap<String, Box<Value>>,
    pub list: *mut Vec<Box<Value>>,
}

#[derive(Copy, Clone, PartialEq, Debug)]
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
            ObjectType::String | ObjectType::Atom => unsafe { drop((*self.obj).string) },
            ObjectType::List => unsafe { drop((*self.obj).list) },
            ObjectType::Object => unsafe { drop((*self.obj).object) },
        }
        // Drop the object itself
        drop(self.obj);
    }

    pub fn print(&self) -> String {
        match self.obj_type {
            ObjectType::String => unsafe { (*(*self.obj).string).to_owned() },
            ObjectType::Atom => {
                let name = unsafe { (*(*self.obj).string).to_owned() };
                format!(":{}", name)
            }
            ObjectType::List => {
                let list = unsafe { (*(*self.obj).list).to_owned() };
                format!(
                    "[{}]",
                    list.into_iter()
                        .map(|i| i.print())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            ObjectType::Object => {
                let obj = unsafe { (*(*self.obj).object).to_owned() };
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
