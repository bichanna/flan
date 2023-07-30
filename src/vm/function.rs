use std::sync::Arc;

pub struct Function {
    pub params: Vec<Arc<str>>,
    pub rest: Option<Arc<str>>,
    pub bytecode: Vec<u8>,
    pub name: Option<Arc<str>>,
}

impl Function {
    pub fn new(
        params: Vec<Arc<str>>,
        rest: Option<Arc<str>>,
        bytecode: Vec<u8>,
        name: Option<Arc<str>>,
    ) -> Self {
        Self {
            params,
            rest,
            bytecode,
            name,
        }
    }
}
