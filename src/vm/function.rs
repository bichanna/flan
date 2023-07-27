use std::rc::Rc;

use crate::compiler::util::MemorySlice;

pub struct Function {
    pub arity: i32,
    pub rest: bool,
    pub mem_slice: MemorySlice,
    pub name: Rc<str>,
}

impl Function {
    pub fn new(arity: i32, rest: bool, mem_slice: MemorySlice, name: &str) -> Self {
        Self {
            arity,
            rest,
            mem_slice,
            name: Rc::from(name),
        }
    }
}
