use std::ptr;

#[derive(Copy, Clone)]
pub struct Function {
    /// Number of names of the parameters of the function
    pub params: usize,
    /// Whether the name of the rest parameter if the function has one
    pub rest: bool,
    /// Address of the function in the bytecode
    pub addr: *const u8,
    /// Path index used for reporting errors
    pub path_idx: usize,
}

impl Function {
    pub fn new(params: usize, rest: bool, path_idx: usize) -> Self {
        Self {
            params,
            rest,
            path_idx,
            addr: ptr::null(),
        }
    }

    /// Sets the address of the function in the bytecode
    pub fn set_addr(&mut self, addr: *const u8) {
        self.addr = addr;
    }
}
