#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Empty,
    Null,
    Str(String),
    Num(f64),
    Bool(bool),
    Atom(String),
    List(Vec<Box<Value>>),
    Object((Vec<String>, Vec<Box<Value>>)),
}

impl Value {
    pub fn equal(&self, other: &Value) -> bool {
        if *self == Value::Empty || *other == Value::Empty {
            true
        } else {
            self == other
        }
    }

    pub fn string(&self, inside: bool) -> String {
        match self {
            Value::Empty => String::from("_"),
            Value::Null => String::from("null"),
            Value::Str(v) => {
                if inside {
                    format!("\"{}\"", v)
                } else {
                    v.clone()
                }
            }
            Value::Bool(v) => format!("{}", v),
            Value::Num(v) => format!("{}", v),
            Value::Atom(v) => String::from(":") + &v,
            Value::List(l) => format!(
                "[{}]",
                l.into_iter()
                    .map(|i| i.string(true))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Value::Object(m) => format!(
                "{}",
                m.0.to_owned()
                    .into_iter()
                    .zip(m.1.to_owned().into_iter())
                    .map(|(k, v)| format!("{}: {}", k, v.string(true)))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}
