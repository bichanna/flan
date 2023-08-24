use super::gc::heap::Object;
use super::value::FFunc;

pub struct Closure {
    /// Pointer to the function
    pub function: Object,
}

impl Closure {
    pub fn new(func: FFunc) -> Self {
        Self { function: func.0 }
    }
}
