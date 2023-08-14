use std::sync::Arc;

use self::native_func::{FNative, NativeFunc};

use super::value::*;
use super::VM;

pub mod native_func;

/// Binds a native function to a global variable
pub fn define_native(vm: &mut VM, name: Arc<str>, func: NativeFunc) {
    vm.globals.insert(name, (FNative::build(func), false));
}
