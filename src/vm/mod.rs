use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::mem::replace;
use std::sync::Arc;

use num_traits::FromPrimitive;

use crate::compiler::opcode::OpCode;
use crate::compiler::util::{from_little_endian, from_little_endian_u32, MemorySlice};
use crate::debug::Debug;
use crate::error::Positions;
use crate::{as_t, force_as_t};

use self::value::*;

pub mod value;

macro_rules! read_byte {
    ($self: expr) => {{
        $self.ip = unsafe { $self.ip.add(1) };
        unsafe { *$self.ip }
    }};
}

macro_rules! read_2bytes {
    ($self: expr) => {
        from_little_endian([read_byte!($self), read_byte!($self)])
    };
}

macro_rules! read_4bytes {
    ($self: expr) => {
        from_little_endian_u32([
            read_byte!($self),
            read_byte!($self),
            read_byte!($self),
            read_byte!($self),
        ])
    };
}

macro_rules! try_push {
    ($self: expr, $val: expr) => {
        match $val {
            Ok(v) => $self.push(v),
            Err(_msg) => {} // TODO: report an error
        }
    };
}

macro_rules! binary_op {
    ($self: expr, $op: tt) => {
        let right = $self.pop();
        let left = $self.pop();
        try_push!($self, left $op right);
    };
}

struct VM<'a> {
    /// Constants
    constants: Vec<Value>,
    /// Positions for error reporting
    positions: Positions,
    /// Instruction pointer, holds the current instruction being executed
    ip: *const u8,
    /// Dynamically sized stack
    stack: Vec<Value>,
    /// All global variables are stored in here
    globals: HashMap<String, Value>,
    /// Debugger for the VM
    debugger: Debug<'a>,
}

impl<'a> VM<'a> {
    pub fn execute(mem_slice: MemorySlice) {
        let mut vm = VM {
            constants: mem_slice.constants.clone(),
            positions: mem_slice.positions.clone(),
            ip: mem_slice.bytecode.as_ptr(),
            stack: Vec::with_capacity(u8::MAX as usize),
            globals: HashMap::with_capacity(12),
            debugger: Debug::new(&mem_slice),
        };
        vm._execute();
    }

    fn _execute(&mut self) {
        let mut inst: OpCode = FromPrimitive::from_u8(unsafe { *self.ip }).unwrap();
        loop {
            #[cfg(feature = "debug")]
            {
                println!("    stack: ");
                self.stack.iter().for_each(|v| println!("[{v}]"));
                println!("");
                self.debugger.disassemble_instruction()
            }

            match inst {
                OpCode::Return => break,

                OpCode::LoadConst => {
                    let val = self.read_const(false);
                    self.push(val);
                }

                OpCode::LoadLongConst => {
                    let val = self.read_const(true);
                    self.push(val);
                }

                OpCode::Negate => {
                    let val = self.pop();
                    if let Ok(num) = -val {
                        self.push(num);
                    } else {
                        // TODO: report an error
                    }
                }

                OpCode::NegateBool => {
                    let val = self.pop();
                    if let Ok(num) = !val {
                        self.push(num);
                    } else {
                        // TODO: report an error
                    }
                }

                OpCode::Add => {
                    binary_op!(self, +);
                }

                OpCode::Sub => {
                    binary_op!(self, -);
                }

                OpCode::Mult => {
                    binary_op!(self, *);
                }

                OpCode::Div => {
                    binary_op!(self, /);
                }

                OpCode::Rem => {
                    binary_op!(self, %);
                }

                OpCode::Pop => {
                    self.pop();
                }

                OpCode::PopN => {
                    let n = read_byte!(self) as i32;
                    self.popn(n);
                }

                OpCode::InitList => {
                    let len = read_2bytes!(self) as usize;
                    let mut list: Vec<Value> = Vec::with_capacity(len);
                    // adding elements to the list
                    (0..len).for_each(|_| list.push(self.pop()));
                    list.reverse();
                    self.push(FList::new(list));
                }

                OpCode::InitObj => {
                    let len = read_2bytes!(self) as usize;
                    let mut obj: HashMap<Arc<str>, Value> = HashMap::with_capacity(len);
                    (0..len).for_each(|_| {
                        // getting the value
                        let val = self.pop();
                        // getting the key
                        let key = self.pop();
                        if let Some(key) = as_t!(key, FVar) {
                            obj.insert(key.0.clone(), val);
                        } else {
                            // TODO: report error
                        }
                    });
                    self.push(FObj::new(obj));
                }

                OpCode::PopExceptLast => {
                    let last = self.pop();
                    self.pop();
                    self.push(last);
                }

                OpCode::PopExceptLastN => {
                    let last = self.pop();
                    let n = read_byte!(self) as i32;
                    self.popn(n);
                    self.push(last);
                }

                OpCode::Jump => {
                    let jump = read_2bytes!(self);
                    unsafe { self.ip.add(jump as usize) };
                }

                OpCode::LongJump => {
                    let jump = read_4bytes!(self);
                    unsafe { self.ip.add(jump as usize) };
                }

                OpCode::JumpIfFalse => {
                    let jump = read_2bytes!(self);
                    if !self.pop().truthy() {
                        unsafe { self.ip.add(jump as usize) };
                    }
                }

                OpCode::Equal => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::new(left.equal(&right)));
                }

                OpCode::NotEqual => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::new(!left.equal(&right)));
                }

                OpCode::GT => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::new(left.greater_than(&right)));
                }

                OpCode::LT => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::new(left.less_than(&right)));
                }

                OpCode::GTEq => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::new(left.greater_than_or_eq(&right)));
                }

                OpCode::LTEq => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::new(left.less_than_or_eq(&right)));
                }

                OpCode::And => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::new(left.truthy() && right.truthy()));
                }

                OpCode::Or => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::new(left.truthy() || right.truthy()));
                }

                OpCode::LoadInt0 => {
                    self.push(FInt::new(0));
                }

                OpCode::LoadInt1 => {
                    self.push(FInt::new(1));
                }

                OpCode::LoadInt2 => {
                    self.push(FInt::new(2));
                }

                OpCode::LoadInt3 => {
                    self.push(FInt::new(3));
                }

                OpCode::LoadTrue => {
                    self.push(FBool::new(true));
                }

                OpCode::LoadFalse => {
                    self.push(FBool::new(false));
                }

                OpCode::LoadEmpty => {
                    self.push(FEmpty::new());
                }

                OpCode::LoadNil => {
                    self.push(FNil::new());
                }

                OpCode::DefGlobal => {
                    self.define_or_set(&VM::define_global);
                }

                OpCode::SetGlobal => {
                    self.define_or_set(&VM::set_global);
                }

                OpCode::GetGlobal => {
                    let v = self.pop();
                    let v = force_as_t!(v, FVar);
                    if let Some(val) = self.globals.get(&v.0.to_string()) {
                        self.push(val.clone());
                    } else {
                        // TODO: report an error
                    }
                }

                OpCode::DefLocal => {
                    fn def_local(vm: &mut VM, _: String, val: Value) {
                        vm.push(val);
                    }
                    self.define_or_set(&def_local);
                }

                OpCode::GetLocal => {
                    let len = self.stack.len();
                    let val = self.stack[len - read_byte!(self) as usize - 1].clone();
                    self.push(val);
                }

                OpCode::SetLocalVar => {
                    let right = self.pop();
                    let idx = read_byte!(self) as usize;
                    self.stack_assign(idx, right.clone());
                    self.push(right)
                }

                OpCode::SetLocalList => {
                    let right = self.pop();
                    let len = read_byte!(self) as usize;
                    let slots = (0..len)
                        .map(|_| (read_byte!(self) == 0, read_byte!(self) as usize))
                        .rev()
                        .collect::<Vec<(bool, usize)>>();

                    if as_t!(right, FList).is_none() {
                        // TODO: report an error
                    }

                    let right_list = &force_as_t!(right, FList).0;

                    if right_list.len() != slots.len() {
                        // TODO: report an error
                    }

                    slots.iter().zip(right_list.iter()).for_each(|(slot, val)| {
                        if slot.0 {
                            self.stack_assign(slot.1, val.clone());
                        }
                    });

                    self.push(right.clone());
                }

                OpCode::SetLocalObj => {
                    let len = read_byte!(self);
                    let slots = (0..len)
                        .map(|_| read_byte!(self) as usize)
                        .rev()
                        .collect::<Vec<usize>>();
                    let left_keys = (0..len)
                        .map(|_| force_as_t!(self.pop(), FVar).0.clone())
                        .rev()
                        .collect::<Vec<Arc<str>>>();
                    let right = self.pop();

                    if as_t!(right, FObj).is_none() {
                        // TODO: report an error
                    }

                    let right_obj = &force_as_t!(right, FObj).0;
                    if right_obj.len() < slots.len() {
                        // TODO: report an error
                    }

                    // actually doing the reassignments
                    slots.iter().zip(left_keys.iter()).for_each(|(slot, key)| {
                        if right_obj.contains_key(key) {
                            self.stack_assign(*slot, right_obj[key].clone());
                        } else {
                            // TODO: report an error
                        }
                    });

                    self.push(right);
                }

                _ => {}
            }

            inst = FromPrimitive::from_u8(read_byte!(self)).unwrap();
        }
    }

    /// A short cut for random access stack assignment
    fn stack_assign(&mut self, idx: usize, val: Value) {
        let len = self.stack.len();
        self.stack[len - idx - 1] = val;
    }

    /// Defines or sets global or local variables
    fn define_or_set(&mut self, func: &dyn Fn(&mut Self, String, Value)) {
        let right = self.pop();
        let left = self.pop();
        if let Some(var) = as_t!(left, FVar) {
            func(self, var.0.to_string(), right.clone());
        } else if let Some(left) = as_t!(left, FList) {
            if let Some(right) = as_t!(right, FList) {
                if right.0.len() != left.0.len() {
                    // TODO: report an error
                } else {
                    left.0.iter().zip(right.0.iter()).for_each(|(l, r)| {
                        if let Some(v) = as_t!(l, FVar) {
                            func(self, v.0.to_string(), r.clone());
                        } else if as_t!(l, FEmpty).is_some() {
                            {} // do nothing
                        } else {
                            // TODO: report an error
                        }
                    });
                }
            } else {
                // TODO: report an error
            }
        } else if let Some(left) = as_t!(left, FObj) {
            if let Some(right) = as_t!(right, FObj) {
                if right.0.len() != left.0.len() {
                    // TODO: report an error
                } else {
                    left.0.iter().for_each(|(key, assignee)| {
                        if let Some(val) = right.0.get(key) {
                            if let Some(var) = as_t!(assignee, FVar) {
                                func(self, var.0.to_string(), val.clone());
                            } else if as_t!(assignee, FEmpty).is_some() {
                                {} // do nothing
                            } else {
                                // TODO: report an error
                            }
                        }
                    });
                }
            } else {
                // TODO: report an error
            }
        } else {
            // TODO: report an error
        }

        self.push(right);
    }

    /// Binds the given value to a global variable name
    fn define_global(vm: &mut VM, name: String, val: Value) {
        if let Entry::Vacant(e) = vm.globals.entry(name) {
            e.insert(val);
        } else {
            // TODO: report an error
        }
    }

    /// Rebinds a new value to a global variable
    fn set_global(vm: &mut VM, name: String, val: Value) {
        if let Entry::Occupied(mut o) = vm.globals.entry(name) {
            o.insert(val);
        } else {
            // TODO: report an error
        }
    }

    /// Returns a Value from `constants`
    fn read_const(&mut self, long: bool) -> Value {
        let idx = if long {
            read_2bytes!(self) as usize
        } else {
            read_byte!(self) as usize
        };

        replace(&mut self.constants[idx], FEmpty::new())
    }

    /// Pops a `Value` off from `stack`
    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    /// Pops `Value`'s `n` times off `stack`
    fn popn(&mut self, n: i32) {
        (0..n).for_each(|_| {
            self.stack.pop();
        });
    }

    /// Pushes a `Value` onto `stack`
    fn push(&mut self, val: Value) {
        self.stack.push(val);

        // growing the stack
        if self.stack.capacity() == self.stack.len() {
            self.stack.reserve(self.stack.len() / 3);
        }
    }
}
