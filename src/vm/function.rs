use std::ptr;

#[derive(Clone)]
pub struct Function {
    /// Number of names of the parameters of the function
    pub params: usize,
    /// Whether the name of the rest parameter if the function has one
    pub rest: bool,
    /// Address of the function in the bytecode
    pub addr: *const u8,
}

impl Function {
    pub fn new(params: usize, rest: bool) -> Self {
        Self {
            params,
            rest,
            addr: ptr::null(),
        }
    }

    pub fn set_addr(&mut self, addr: *const u8) {
        self.addr = addr;
    }
}
