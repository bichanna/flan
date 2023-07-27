use std::rc::Rc;

use crate::compiler::util::MemorySlice;

pub enum FuncType {
    Function,
    Script,
}

pub struct Function {
    pub params: Vec<Rc<str>>,
    pub rest: Option<Rc<str>>,
    pub mem_slice: MemorySlice,
    pub name: Rc<str>,
}

impl Function {
    pub fn new(
        params: Vec<Rc<str>>,
        rest: Option<Rc<str>>,
        mem_slice: MemorySlice,
        name: &str,
    ) -> Self {
        Self {
            params,
            rest,
            mem_slice,
            name: Rc::from(name),
        }
    }
}
