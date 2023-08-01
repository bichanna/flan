use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::mem::replace;
use std::sync::Arc;

use num_traits::FromPrimitive;

use crate::compiler::opcode::OpCode;
use crate::compiler::test_compile;
use crate::compiler::util::{from_little_endian, from_little_endian_u32, MemorySlice};
use crate::debug::Debug;
use crate::error::Positions;
use crate::{as_t, force_as_t};

use self::gc::heap::Heap;
use self::value::*;

pub mod function;
pub mod gc;
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

macro_rules! partial_match {
    ($self: expr, $target: expr, $jump: expr, $cond: expr, $type: ty, $rn: expr) => {
        if as_t!($target, FEmpty).is_some() {
            {} // do nothing
        } else if let Some(target) = as_t!($target, $type) {
            if target.0 != $cond.0 {
                $self.jump($jump);
                *$rn = true;
            }
        } else if as_t!($target, FVar).is_some() {
            // TODO: fix this later
        } else {
            $self.jump($jump);
            *$rn = true;
        }
    };
}

pub struct VM<'a> {
    /// Heap
    heap: &'a mut Heap,
    /// Constants
    constants: Vec<Value>,
    /// Positions for error reporting
    positions: Positions,
    /// Instruction pointer, holds the current instruction being executed
    ip: *const u8,
    /// Dynamically sized stack
    stack: Vec<Value>,
    /// All global variables are stored in here
    globals: HashMap<Arc<str>, Value>,
    /// Debugger for the VM
    #[cfg(feature = "debug")]
    debugger: Debug<'a>,
}

impl<'a> VM<'a> {
    pub fn execute(mem_slice: MemorySlice, heap: &'a mut Heap) {
        let mut vm = VM {
            heap,
            constants: mem_slice.constants.clone(),
            positions: mem_slice.positions.clone(),
            ip: mem_slice.bytecode.as_ptr(),
            stack: Vec::with_capacity(u8::MAX as usize),
            globals: HashMap::with_capacity(12),
            #[cfg(feature = "debug")]
            debugger: Debug::new(&mem_slice),
        };
        vm._execute();
    }

    fn _execute(&mut self) {
        let mut inst: OpCode = FromPrimitive::from_u8(unsafe { *self.ip }).unwrap();
        loop {
            #[cfg(feature = "debug")]
            {
                print!("    stack: ");
                self.stack.iter().for_each(|v| print!("[{v}]"));
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
                    let len = read_byte!(self) as usize;
                    let mut list: Vec<Value> = Vec::with_capacity(len);
                    // adding elements to the list
                    (0..len).for_each(|_| list.push(self.pop()));
                    list.reverse();
                    let flist = FList::new(self.heap, list);
                    self.push(flist);
                }

                OpCode::InitObj => {
                    let len = read_byte!(self) as usize;
                    let mut obj: HashMap<Arc<str>, Value> = HashMap::with_capacity(len);
                    (0..len).for_each(|_| {
                        // getting the key
                        let key = self.pop();
                        // getting the value
                        let val = self.pop();

                        if let Some(key) = as_t!(key, FVar) {
                            obj.insert(key.0.clone(), val);
                        } else {
                            // TODO: report error
                        }
                    });
                    let fobj = FObj::new(self.heap, obj);
                    self.push(fobj);
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
                    self.jump(jump as usize);
                }

                OpCode::LongJump => {
                    let jump = read_4bytes!(self);
                    self.jump(jump as usize);
                }

                OpCode::JumpIfFalse => {
                    let jump = read_2bytes!(self);
                    if !self.pop().truthy() {
                        self.jump(jump as usize);
                        continue;
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
                    if let Some(val) = self.globals.get(v.0.as_ref()) {
                        self.push(val.clone());
                    } else {
                        // TODO: report an error
                    }
                }

                OpCode::DefLocal => {
                    fn def_local(vm: &mut VM, _: Arc<str>, val: Value) {
                        vm.push(val);
                    }
                    self.define_or_set(&def_local);
                }

                OpCode::GetLocal => {
                    let idx = read_byte!(self) as usize;
                    let val = self.stack[idx].clone();
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

                    let right_list = &force_as_t!(right, FList).inner();

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

                    let right_obj = &force_as_t!(right, FObj).inner();
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

                OpCode::Match => {
                    let target = self.pop();
                    let cond = self.pop();
                    let jump = read_4bytes!(self);
                    let has_next = read_byte!(self) == 1;
                    let mut is_body_running = false;

                    // recalculating the jump
                    let jump = if has_next { jump - 1 + 5 } else { jump - 1 } as usize;

                    fn match_expr(
                        vm: &mut VM,
                        cond: Value,
                        target: Value,
                        jump: usize,
                        is_body_running: &mut bool,
                    ) {
                        if as_t!(cond, FEmpty).is_some() {
                            {} // do nothing
                        } else if let Some(int) = as_t!(cond, FInt) {
                            partial_match!(vm, target, jump, int, FInt, is_body_running);
                        } else if let Some(float) = as_t!(cond, FFloat) {
                            partial_match!(vm, target, jump, float, FFloat, is_body_running);
                        } else if let Some(b) = as_t!(cond, FBool) {
                            partial_match!(vm, target, jump, b, FBool, is_body_running);
                        } else if let Some(atom) = as_t!(cond, FAtom) {
                            partial_match!(vm, target, jump, atom, FAtom, is_body_running);
                        } else if as_t!(cond, FNil).is_some() {
                            if as_t!(target, FNil).is_some() || as_t!(target, FEmpty).is_some() {
                                {} // do nothing
                            } else if as_t!(target, FVar).is_some() {
                                // TODO: fix this later
                            } else {
                                vm.jump(jump);
                                *is_body_running = true;
                            }
                        } else if let Some(string) = as_t!(cond, FStr) {
                            if as_t!(target, FEmpty).is_some() {
                                {} // do nothing
                            } else if let Some(t_str) = as_t!(target, FStr) {
                                if t_str.inner() != string.inner() {
                                    vm.jump(jump);
                                    *is_body_running = true;
                                }
                            } else if as_t!(target, FVar).is_some() {
                                // TODO: fix this later
                            } else {
                                vm.jump(jump);
                                *is_body_running = true;
                            }
                        } else if let Some(flist) = as_t!(cond, FList) {
                            let list = flist.inner();
                            if as_t!(target, FEmpty).is_some() {
                                {} // do nothing
                            } else if as_t!(target, FVar).is_some() {
                                // TODO: fix this later
                            } else if let Some(t_flist) = as_t!(target, FList) {
                                let t_list = t_flist.inner();
                                if list.len() != t_list.len() {
                                    // TODO: report an error
                                }
                                list.iter().zip(t_list.iter()).for_each(|(l, r)| {
                                    match_expr(vm, l.clone(), r.clone(), jump, is_body_running);
                                });
                            } else {
                                vm.jump(jump);
                                *is_body_running = true;
                            }
                        } else if let Some(fobj) = as_t!(cond, FObj) {
                            let obj = fobj.inner();
                            if as_t!(target, FEmpty).is_some() {
                                {} // do nothing
                            } else if as_t!(target, FVar).is_some() {
                                // TODO: fix this later
                            } else if let Some(t_fobj) = as_t!(target, FObj) {
                                let t_obj = t_fobj.inner();
                                if t_obj.len() != obj.len() {
                                    // TODO: report an error
                                }
                                obj.iter().for_each(|(l_key, l_val)| {
                                    if t_obj.contains_key(l_key) {
                                        let t_val = t_obj.get(l_key).unwrap().clone();
                                        match_expr(vm, l_val.clone(), t_val, jump, is_body_running);
                                    }
                                });
                            }
                        }
                    }

                    match_expr(self, cond.clone(), target, jump, &mut is_body_running);

                    if is_body_running && has_next {
                        self.push(cond);
                    }
                }

                OpCode::Call => {}

                OpCode::GetProperty => {
                    let attr = self.pop();
                    let inst = self.pop();

                    if let Some(flist) = as_t!(inst, FList) {
                        let list = flist.inner();
                        if let Some(idx) = as_t!(attr, FInt) {
                            let idx = idx.0 as usize;
                            if let Some(val) = list.get(idx) {
                                self.push(val.clone());
                            } else {
                                // TODO: report an error
                            }
                        } else {
                            // TODO: report an error
                        }
                    } else if let Some(fobj) = as_t!(inst, FObj) {
                        let obj = fobj.inner();
                        if let Some(key) = as_t!(attr, FVar) {
                            if let Some(val) = obj.get(&key.0) {
                                self.push(val.clone());
                            } else {
                                // TODO: report an error
                            }
                        } else {
                            // TODO: report an error
                        }
                    } else if let Some(fstr) = as_t!(inst, FStr) {
                        let string = fstr.inner();
                        if let Some(idx) = as_t!(attr, FInt) {
                            let idx = idx.0 as usize;
                            if idx >= string.len() {
                                // TODO: report an error
                            }

                            let new_str =
                                FStr::new(self.heap, string.chars().nth(idx).unwrap().to_string());
                            self.push(new_str);
                        } else if let Some(flist) = as_t!(attr, FList) {
                            let list = flist.inner();
                            match list.len() {
                                0 => {
                                    let new_str = string.clone();
                                    let new_fstr = FStr::new(self.heap, new_str);
                                    self.push(new_fstr);
                                }
                                1 => {
                                    if let Some(l) = as_t!(list[0], FInt) {
                                        let l = l.0 as usize;

                                        if l >= string.len() {
                                            // TODO: report an error
                                        }

                                        let slice = &string[l..];
                                        let new_fstr = FStr::new(self.heap, slice.to_string());
                                        self.push(new_fstr);
                                    } else {
                                        // TODO: report an error
                                    }
                                }
                                2 => {
                                    if as_t!(list[0], FInt).is_some()
                                        && as_t!(list[1], FInt).is_some()
                                    {
                                        let l0 = force_as_t!(list[0], FInt).0 as usize;
                                        let l1 = force_as_t!(list[1], FInt).0 as usize;

                                        if l0 >= string.len() || l1 >= string.len() {
                                            // TODO: report an error
                                        }

                                        let slice = &string[l0..l1];
                                        let new_fstr = FStr::new(self.heap, slice.to_string());
                                        self.push(new_fstr);
                                    } else {
                                        // TODO: report an error
                                    }
                                }
                                _ => {} // TODO: report an error
                            }
                        }
                    } else {
                        // TODO: report an error
                    }
                }

                OpCode::SetProperty => {
                    let val = self.pop();
                    let attr = self.pop();
                    let inst = self.pop();

                    if let Some(flist) = as_t!(inst, FList) {
                        let list = unsafe { flist.inner_mut().as_mut().unwrap() };
                        if let Some(idx) = as_t!(attr, FInt) {
                            let idx = idx.0 as usize;
                            if idx >= list.len() {
                                // TODO: report an error
                            }
                            list[idx] = val;
                        } else {
                            // TODO: report an error
                        }
                    } else if let Some(fobj) = as_t!(inst, FObj) {
                        let obj = unsafe { fobj.inner_mut().as_mut().unwrap() };
                        if let Some(key) = as_t!(attr, FVar) {
                            obj.insert(key.0.clone(), val);
                        } else {
                            // TODO: report an error
                        }
                    } else {
                        // TODO: report an error
                    }
                }
            }

            inst = FromPrimitive::from_u8(read_byte!(self)).unwrap();
        }
    }

    fn jump(&mut self, jmp: usize) {
        #[cfg(feature = "debug")]
        {
            self.debugger.offset += jmp;
        }
        self.ip = unsafe { self.ip.add(jmp) };
    }

    /// A short cut for random access stack assignment
    fn stack_assign(&mut self, idx: usize, val: Value) {
        let len = self.stack.len();
        self.stack[len - idx - 1] = val;
    }

    /// Defines or sets global or local variables
    fn define_or_set(&mut self, func: &dyn Fn(&mut Self, Arc<str>, Value)) {
        let right = self.pop();
        let left = self.pop();
        if let Some(var) = as_t!(left, FVar) {
            func(self, var.0.clone(), right.clone());
        } else if let Some(left) = as_t!(left, FList) {
            if let Some(right) = as_t!(right, FList) {
                if right.inner().len() != left.inner().len() {
                    // TODO: report an error
                } else {
                    left.inner()
                        .iter()
                        .zip(right.inner().iter())
                        .for_each(|(l, r)| {
                            if let Some(v) = as_t!(l, FVar) {
                                func(self, v.0.clone(), r.clone());
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
                if right.inner().len() != left.inner().len() {
                    // TODO: report an error
                } else {
                    left.inner().iter().for_each(|(key, assignee)| {
                        if let Some(val) = right.inner().get(key) {
                            if let Some(var) = as_t!(assignee, FVar) {
                                func(self, var.0.clone(), val.clone());
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
    fn define_global(vm: &mut VM, name: Arc<str>, val: Value) {
        if let Entry::Vacant(e) = vm.globals.entry(name) {
            e.insert(val);
        } else {
            // TODO: report an error
        }
    }

    /// Rebinds a new value to a global variable
    fn set_global(vm: &mut VM, name: Arc<str>, val: Value) {
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

        replace(&mut self.constants[idx], FNil::new())
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

pub fn test_execute(src: &str) {
    let (mem_slice, mut heap) = test_compile(src);

    Debug::run("TEST!", &mem_slice);

    VM::execute(mem_slice, &mut heap);

    heap.deallocate_all();
}
