use std::collections::HashMap;
#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Box<Value>>),
    Obj(HashMap<String, Box<Value>>),
}

impl Value {
    pub fn print(&self) -> String {
        match self {
            Value::Bool(v) => format!("{}", v),
            Value::Int(v) => format!("{}", v),
            Value::Float(v) => format!("{}", v),
            Value::Str(v) => format!("{}", v),
            Value::List(list) => format!(
                "[{}]",
                list.into_iter()
                    .map(|x| x.print())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Value::Obj(obj) => {
                format!(
                    "{{{}}}",
                    obj.into_iter()
                        .map(|(k, v)| format!("{}:{}", k, v.print()))
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
        }
    }
}
