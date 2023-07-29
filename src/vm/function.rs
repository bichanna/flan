use std::rc::Rc;

pub struct Function {
    pub params: Vec<Rc<str>>,
    pub rest: Option<Rc<str>>,
    pub bytecode: Vec<u8>,
    pub name: Option<Rc<str>>,
}

impl Function {
    pub fn new(
        params: Vec<Rc<str>>,
        rest: Option<Rc<str>>,
        bytecode: Vec<u8>,
        name: Option<Rc<str>>,
    ) -> Self {
        Self {
            params,
            rest,
            bytecode,
            name,
        }
    }
}
