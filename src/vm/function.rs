#[derive(Debug, Clone, Copy)]
pub enum FuncType {
    Function,
    Script,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub arity: usize,
    pub bytecode: Vec<u8>,
    pub name: String,
}

impl Function {
    pub fn new(name: String) -> Self {
        Self {
            arity: 0,
            name,
            bytecode: Vec::with_capacity(15),
        }
    }
}
