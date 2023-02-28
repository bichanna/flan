use std::collections::HashMap;

pub enum Assignee {
    List(Vec<String>),
    Obj(HashMap<String, (String, u8)>),
    Var(String),
}
