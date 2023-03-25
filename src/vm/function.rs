use super::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub arity: usize,
    /// The compiled bytecode
    pub bytecode: Vec<u8>,
    /// Function name
    pub name: Option<String>,
    /// For simplicity's sake, we'll put all constants in here
    pub values: Vec<Value>,
}

#[derive(Debug, Clone, Copy)]
pub enum FuncType {
    TopLevel,
    Function,
}

impl Function {
    pub fn new(name: Option<String>) -> Self {
        Self {
            arity: 0,
            bytecode: Vec::with_capacity(32),
            name,
            values: Vec::with_capacity(12),
        }
    }
}
