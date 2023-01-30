use std::collections::HashMap;

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
}

#[derive(Copy, Clone)]
pub struct Object {
    obj_type: ObjectType,
    obj: ObjectUnion,
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
