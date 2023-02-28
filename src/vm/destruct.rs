use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Assignee {
    List(Vec<String>),
    Obj(HashMap<String, (String, u8)>),
    Var(String),
}
