use std::ptr;
use std::sync::Arc;

pub struct Function {
    /// Names of the parameters of the function
    pub params: Vec<Arc<str>>,
    /// Name of the rest parameter if the function has one
    pub rest: Option<Arc<str>>,
    /// Name of the function if it has one
    pub name: Option<Arc<str>>,
    /// Address of the function in the bytecode
    pub addr: *const u8,
}

impl Function {
    pub fn new(params: Vec<Arc<str>>, rest: Option<Arc<str>>, name: Option<Arc<str>>) -> Self {
        Self {
            params,
            rest,
            name,
            addr: ptr::null(),
        }
    }

    pub fn set_addr(&mut self, addr: *const u8) {
        self.addr = addr;
    }
}
