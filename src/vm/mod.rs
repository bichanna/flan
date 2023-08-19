use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::mem::replace;
use std::sync::Arc;

use arrayvec::ArrayVec;
use num_traits::FromPrimitive;

use crate::compiler::opcode::OpCode;
use crate::compiler::test_compile;
use crate::compiler::util::{from_little_endian, from_little_endian_u32, MemorySlice};
use crate::debug::Debug;
use crate::error::{ErrType, Node, Positions, Stack};
use crate::*;

use self::function::Function;
use self::gc::heap::Heap;
use self::native::native_func::FNative;
use self::value::*;

pub mod function;
pub mod gc;
pub mod native;
mod util_macro;
pub mod value;

const U8_MAX: usize = u8::MAX as usize;
const FRAME_MAX: usize = 64;
const STACK_MAX: usize = FRAME_MAX * U8_MAX;

type DefSetFunc<'a> = &'a dyn Fn(&mut VM, Arc<str>, Value, bool);

struct CallFrame {
    /// Stores the information about the function
    func: Function,
    /// Instruction pointer, holds the current instruction being executed
    ip: *const u8,

    slot_bottom: usize,
    slot_count: usize,
}

pub struct VM<'a> {
    /// Pointer to the first instruction
    first_ip: *const u8,
    /// Heap
    heap: &'a mut Heap,
    /// Constants
    constants: Vec<Value>,
    /// Positions for error reporting
    positions: Positions,
    /// Stack
    stack: ArrayVec<Value, STACK_MAX>,
    /// All global variables are stored in here. The second value in the tuple represents whether
    /// the variable is mutable or not
    globals: HashMap<Arc<str>, (Value, bool)>,
    /// Call frames
    frames: ArrayVec<CallFrame, FRAME_MAX>,

    /// Debugger for the VM
    #[cfg(feature = "debug")]
    debugger: Debug<'a>,
}

impl<'a> VM<'a> {
    pub fn execute(mem_slice: MemorySlice, heap: &'a mut Heap) {
        let stack: ArrayVec<Value, STACK_MAX> = ArrayVec::new();

        let ip = mem_slice.bytecode.as_ptr();

        let mut frames: ArrayVec<CallFrame, FRAME_MAX> = ArrayVec::new();
        frames.push(CallFrame {
            ip,
            func: Function::new(0, false, 0),
            slot_bottom: 0,
            slot_count: 0,
        });

        let mut vm = VM {
            first_ip: ip,
            heap,
            constants: mem_slice.constants.clone(),
            positions: mem_slice.positions.clone(),
            stack,
            globals: HashMap::with_capacity(12),
            frames,

            #[cfg(feature = "debug")]
            debugger: Debug::new(&mem_slice),
        };
        vm._execute();
    }

    fn _execute(&mut self) {
        let mut inst: OpCode = FromPrimitive::from_u8(unsafe { *current_frame!(self).ip }).unwrap();
        loop {
            #[cfg(feature = "debug")]
            {
                print!("    stack: ");
                self.stack.iter().for_each(|v| print!("[{v}]"));
                println!("");
                self.debugger.disassemble_instruction()
            }

            match inst {
                OpCode::Halt => {
                    break;
                }

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
                    match -val {
                        Ok(num) => self.push(num),
                        Err(err) => self.runtime_err(err),
                    }
                }

                OpCode::NegateBool => {
                    let val = self.pop();
                    match !val {
                        Ok(val) => self.push(val),
                        Err(err) => self.runtime_err(err),
                    }
                }

                OpCode::Const => {
                    let val = self.pop();
                    if let Some(fstr) = as_t!(val, FStr) {
                        unsafe { (*fstr.inner_mut()).1 = false };
                    } else if let Some(flist) = as_t!(val, FList) {
                        unsafe { (*flist.inner_mut()).1 = false };
                    } else if let Some(fobj) = as_t!(val, FObj) {
                        unsafe { (*fobj.inner_mut()).1 = false };
                    }
                    self.push(val);
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

                OpCode::InitTup => {
                    let len = read_byte!(self) as usize;
                    // adding each elements to the tuple
                    let mut tup = (0..len).map(|_| self.pop()).collect::<Vec<Value>>();
                    tup.reverse();
                    let ftup = FTup::build(tup.into());
                    self.push(ftup);
                }

                OpCode::InitList => {
                    let len = read_byte!(self) as usize;
                    let mutable = read_byte!(self) == 1;
                    // creating the list
                    let mut list = (0..len).map(|_| self.pop()).collect::<Vec<Value>>();
                    list.reverse();
                    let flist = FList::build(self.heap, list, mutable);
                    self.push(flist);
                }

                OpCode::InitObj => {
                    let len = read_byte!(self) as usize;
                    let mutable = read_byte!(self) == 1;
                    let obj = (0..len)
                        .map(|_| {
                            let key = self.pop();
                            // getting the key
                            let key = force_as_t!(key, FVar);
                            // getting the value
                            let val = self.pop();

                            (key.0.clone(), val)
                        })
                        .collect::<HashMap<Arc<str>, Value>>();
                    let fobj = FObj::build(self.heap, obj, mutable);
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
                    self.push(FBool::build(left.equal(&right)));
                }

                OpCode::NotEqual => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::build(!left.equal(&right)));
                }

                OpCode::GT => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::build(left.greater_than(&right)));
                }

                OpCode::LT => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::build(left.less_than(&right)));
                }

                OpCode::GTEq => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::build(left.greater_than_or_eq(&right)));
                }

                OpCode::LTEq => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::build(left.less_than_or_eq(&right)));
                }

                OpCode::And => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::build(left.truthy() && right.truthy()));
                }

                OpCode::Or => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(FBool::build(left.truthy() || right.truthy()));
                }

                OpCode::LoadInt0 => {
                    self.push(FInt::build(0));
                }

                OpCode::LoadInt1 => {
                    self.push(FInt::build(1));
                }

                OpCode::LoadInt2 => {
                    self.push(FInt::build(2));
                }

                OpCode::LoadInt3 => {
                    self.push(FInt::build(3));
                }

                OpCode::LoadTrue => {
                    self.push(FBool::build(true));
                }

                OpCode::LoadFalse => {
                    self.push(FBool::build(false));
                }

                OpCode::LoadEmpty => {
                    self.push(FEmpty::build());
                }

                OpCode::LoadNil => {
                    self.push(FNil::build());
                }

                OpCode::DefGlobal => {
                    let mutability = read_byte!(self) == 1;
                    self.define_or_set(&VM::define_global, mutability);
                }

                OpCode::SetGlobal => {
                    self.define_or_set(&VM::set_global, false);
                }

                OpCode::GetGlobal => {
                    let v = self.pop();
                    let v = force_as_t!(v, FVar);
                    if let Some((val, _)) = self.globals.get(v.0.as_ref()) {
                        self.push(val.clone());
                    } else {
                        self.runtime_err(format!("global variable {} is not defined", v.0));
                    }
                }

                OpCode::DefLocal => {
                    fn def_local(vm: &mut VM, _: Arc<str>, val: Value, _: bool) {
                        vm.slots_push(val);
                    }
                    self.define_or_set(&def_local, false);
                }

                OpCode::GetLocal => {
                    let idx = read_byte!(self) as usize;
                    let val = slot_at_index!(self, idx).clone();
                    self.push(val);
                }

                OpCode::SetLocalVar => {
                    let right = self.pop();
                    let idx = read_byte!(self) as usize;
                    self.slot_assign(idx, right.clone());
                    self.push(right)
                }
                OpCode::SetLocalTup => {
                    let right = self.pop();
                    let len = read_byte!(self) as usize;
                    let slots = (0..len)
                        .map(|_| (read_byte!(self) == 0, read_byte!(self) as usize))
                        .rev()
                        .collect::<Vec<(bool, usize)>>();

                    if as_t!(right, FTup).is_none() {
                        self.runtime_err(format!("expected type tup but got {}", right.type_str()));
                    }

                    let right_tup = &force_as_t!(right, FTup).0;

                    if right_tup.len() != slots.len() {
                        self.runtime_err(format!(
                            "invalid length: {} and {}",
                            slots.len(),
                            right_tup.len()
                        ));
                    }

                    slots.iter().zip(right_tup.iter()).for_each(|(slot, val)| {
                        if slot.0 {
                            self.slot_assign(slot.1, val.clone());
                        }
                    });

                    self.push(right.clone());
                }
                OpCode::SetLocalList => {
                    let right = self.pop();
                    let len = read_byte!(self) as usize;
                    let slots = (0..len)
                        .map(|_| (read_byte!(self) == 0, read_byte!(self) as usize))
                        .rev()
                        .collect::<Vec<(bool, usize)>>();

                    if as_t!(right, FList).is_none() {
                        self.runtime_err(format!(
                            "expected type list but got {}",
                            right.type_str()
                        ));
                    }

                    let right_list = &force_as_t!(right, FList).inner().0;

                    if right_list.len() != slots.len() {
                        self.runtime_err(format!(
                            "invalid length: {} and {}",
                            slots.len(),
                            right_list.len()
                        ));
                    }

                    slots.iter().zip(right_list.iter()).for_each(|(slot, val)| {
                        if slot.0 {
                            self.slot_assign(slot.1, val.clone());
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
                        self.runtime_err(format!(
                            "expected type obj just got {}",
                            right.type_str()
                        ));
                    }

                    let right_obj = &force_as_t!(right, FObj).inner().0;
                    if right_obj.len() < slots.len() {
                        self.runtime_err(format!(
                            "invalid length: {} and {}",
                            slots.len(),
                            right_obj.len()
                        ));
                    }

                    // actually doing the reassignments
                    slots.iter().zip(left_keys.iter()).for_each(|(slot, key)| {
                        if right_obj.contains_key(key) {
                            self.slot_assign(*slot, right_obj[key].clone());
                        } else {
                            self.runtime_err(format!("key '{}' does not exist", key));
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
                            let list = &flist.inner().0;
                            if as_t!(target, FEmpty).is_some() {
                                {} // do nothing
                            } else if as_t!(target, FVar).is_some() {
                                // TODO: fix this later
                            } else if let Some(t_flist) = as_t!(target, FList) {
                                let t_list = &t_flist.inner().0;
                                if list.len() != t_list.len() {
                                    vm.runtime_err(format!(
                                        "invalid length: {} and {}",
                                        list.len(),
                                        t_list.len()
                                    ));
                                }
                                list.iter().zip(t_list.iter()).for_each(|(l, r)| {
                                    match_expr(vm, l.clone(), r.clone(), jump, is_body_running);
                                });
                            } else {
                                vm.jump(jump);
                                *is_body_running = true;
                            }
                        } else if let Some(fobj) = as_t!(cond, FObj) {
                            let obj = &fobj.inner().0;
                            if as_t!(target, FEmpty).is_some() {
                                {} // do nothing
                            } else if as_t!(target, FVar).is_some() {
                                // TODO: fix this later
                            } else if let Some(t_fobj) = as_t!(target, FObj) {
                                let t_obj = &t_fobj.inner().0;
                                if t_obj.len() != obj.len() {
                                    vm.runtime_err(format!(
                                        "invalid length: {} and {}",
                                        obj.len(),
                                        t_obj.len()
                                    ));
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

                OpCode::GetProperty => {
                    let attr = self.pop();
                    let inst = self.pop();

                    if let Some(flist) = as_t!(inst, FList) {
                        let (list, mutable) = flist.inner();
                        if let Some(idx) = as_t!(attr, FInt) {
                            let idx = idx.0 as usize;
                            if let Some(val) = list.get(idx) {
                                self.push(val.clone());
                            } else {
                                self.runtime_err(format!("invalid index {}", idx));
                            }
                        } else if let Some(range) = as_t!(attr, FList) {
                            let range = &range.inner().0;
                            match range.len() {
                                0 => {
                                    let new_list = list.clone();
                                    let new_flist = FList::build(self.heap, new_list, true);
                                    self.push(new_flist);
                                }
                                1 => {
                                    if let Some(l) = as_t!(range[0], FInt) {
                                        let l = l.0 as usize;

                                        if l >= list.len() {
                                            self.runtime_err(format!("invalid index {}", l));
                                        }

                                        let slice = &list[l..];
                                        let new_flist =
                                            FList::build(self.heap, slice.to_vec(), *mutable);
                                        self.push(new_flist);
                                    } else {
                                        self.runtime_err(format!(
                                            "expected type int but got {}",
                                            range[0].type_str()
                                        ));
                                    }
                                }
                                2 => {
                                    if as_t!(range[0], FInt).is_some()
                                        && as_t!(range[1], FInt).is_some()
                                    {
                                        let l0 = force_as_t!(range[0], FInt).0 as usize;
                                        let l1 = force_as_t!(range[1], FInt).0 as usize;

                                        if l0 >= list.len() || l1 >= list.len() || l0 > l1 {
                                            self.runtime_err(format!(
                                                "invalid range [{}, {})",
                                                l0, l1
                                            ));
                                        }

                                        let slice = &list[l0..l1];
                                        let new_flist =
                                            FList::build(self.heap, slice.to_vec(), *mutable);
                                        self.push(new_flist);
                                    } else {
                                        self.runtime_err(format!(
                                            "expected ints but got {} and {}",
                                            range[0].type_str(),
                                            range[1].type_str()
                                        ));
                                    }
                                }
                                _ => {
                                    self.runtime_err(format!("invalid number {}", range.len()));
                                }
                            }
                        } else {
                            self.runtime_err(format!("expected list but got {}", attr.type_str()));
                        }
                    } else if let Some(ftup) = as_t!(inst, FTup) {
                        let tup = &ftup.0;
                        if let Some(idx) = as_t!(attr, FInt) {
                            let idx = idx.0 as usize;
                            if let Some(val) = tup.get(idx) {
                                self.push(val.clone());
                            } else {
                                self.runtime_err(format!("invalid index: {}", idx));
                            }
                        } else if let Some(range) = as_t!(attr, FList) {
                            let range = &range.inner().0;
                            match range.len() {
                                0 => {
                                    // deep cloning the tuple (since tuple is immutable, this is a
                                    // bit redundant)
                                    let new_tup = FTup::build(
                                        tup.iter().cloned().collect::<Vec<Value>>().into(),
                                    );
                                    self.push(new_tup);
                                }
                                1 => {
                                    if let Some(l) = as_t!(range[0], FInt) {
                                        let l = l.0 as usize;

                                        if l >= tup.len() {
                                            self.runtime_err(format!("invalid index {}", l));
                                        }

                                        let slice = &tup[l..];
                                        let new_flist =
                                            FList::build(self.heap, slice.to_vec(), false);
                                        self.push(new_flist);
                                    } else {
                                        self.runtime_err(format!(
                                            "expected type int but got {}",
                                            range[0].type_str(),
                                        ));
                                    }
                                }
                                2 => {
                                    if as_t!(range[0], FInt).is_some()
                                        && as_t!(range[1], FInt).is_some()
                                    {
                                        let l0 = force_as_t!(range[0], FInt).0 as usize;
                                        let l1 = force_as_t!(range[1], FInt).0 as usize;

                                        if l0 >= tup.len() || l1 >= tup.len() || l0 > l1 {
                                            self.runtime_err(format!(
                                                "invalid range [{}, {})",
                                                l0, l1
                                            ));
                                        }

                                        let slice = &tup[l0..l1];
                                        let new_flist =
                                            FList::build(self.heap, slice.to_vec(), false);
                                        self.push(new_flist);
                                    } else {
                                        self.runtime_err(format!(
                                            "expected ints but got {} and {}",
                                            range[0].type_str(),
                                            range[1].type_str()
                                        ));
                                    }
                                }
                                _ => {
                                    self.runtime_err(format!("invalid number {}", range.len()));
                                }
                            }
                        } else {
                            self.runtime_err(format!(
                                "expected either int or list but got {}",
                                attr.type_str()
                            ));
                        }
                    } else if let Some(fobj) = as_t!(inst, FObj) {
                        let obj = &fobj.inner().0;
                        let key = force_as_t!(attr, FVar);
                        if let Some(val) = obj.get(&key.0) {
                            self.push(val.clone());
                        } else {
                            self.runtime_err(format!("key {} does not exist", key.0));
                        }
                    } else if let Some(fstr) = as_t!(inst, FStr) {
                        let (string, mutable) = fstr.inner();
                        if let Some(idx) = as_t!(attr, FInt) {
                            let idx = idx.0 as usize;
                            if idx >= string.len() {
                                self.runtime_err(format!("invalid index {}", idx));
                            }

                            let new_str = FStr::build(
                                self.heap,
                                string.chars().nth(idx).unwrap().to_string(),
                                false,
                            );
                            self.push(new_str);
                        } else if let Some(flist) = as_t!(attr, FList) {
                            let list = &flist.inner().0;
                            match list.len() {
                                0 => {
                                    let new_str = string.clone();
                                    let new_fstr = FStr::build(self.heap, new_str, true);
                                    self.push(new_fstr);
                                }
                                1 => {
                                    if let Some(l) = as_t!(list[0], FInt) {
                                        let l = l.0 as usize;

                                        if l >= string.len() {
                                            self.runtime_err(format!("invalid index {}", l));
                                        }

                                        let slice = &string[l..];
                                        let new_fstr =
                                            FStr::build(self.heap, slice.to_string(), *mutable);
                                        self.push(new_fstr);
                                    } else {
                                        self.runtime_err(format!(
                                            "expected int but got {}",
                                            attr.type_str()
                                        ));
                                    }
                                }
                                2 => {
                                    if as_t!(list[0], FInt).is_some()
                                        && as_t!(list[1], FInt).is_some()
                                    {
                                        let l0 = force_as_t!(list[0], FInt).0 as usize;
                                        let l1 = force_as_t!(list[1], FInt).0 as usize;

                                        if l0 >= string.len() || l1 >= string.len() {
                                            self.runtime_err(format!(
                                                "invalid range [{}, {})",
                                                l0, l1
                                            ));
                                        }

                                        let slice = &string[l0..l1];
                                        let new_fstr =
                                            FStr::build(self.heap, slice.to_string(), *mutable);
                                        self.push(new_fstr);
                                    } else {
                                        self.runtime_err(format!(
                                            "expected ints but got {} and {}",
                                            list[0].type_str(),
                                            list[1].type_str()
                                        ));
                                    }
                                }
                                _ => {
                                    self.runtime_err(format!("invalid number {}", list.len()));
                                }
                            }
                        }
                    } else {
                        self.runtime_err(format!(
                            "expected either list, tuple, or str but got {}",
                            attr.type_str()
                        ));
                    }
                }

                OpCode::SetProperty => {
                    let val = self.pop();
                    let attr = self.pop();
                    let inst = self.pop();

                    if let Some(flist) = as_t!(inst, FList) {
                        let (list, mutable) = unsafe { flist.inner_mut().as_mut().unwrap() };

                        // checking for mutability
                        if !*mutable {
                            self.runtime_err("value is immutable".to_string());
                        }

                        if let Some(idx) = as_t!(attr, FInt) {
                            let idx = idx.0 as usize;
                            if idx >= list.len() {
                                self.runtime_err(format!("invalid index {}", idx));
                            }
                            list[idx] = val.clone();
                        } else if let Some(idxs) = as_t!(attr, FList) {
                            let idxs = unsafe { &(*idxs.inner_mut()).0 };
                            if !idxs.is_empty() {
                                if let Some(val) = as_t!(val, FList) {
                                    let rlist = &val.inner().0;
                                    if rlist.len() != idxs.len() {
                                        self.runtime_err(format!(
                                            "invalid length: {} and {}",
                                            idxs.len(),
                                            rlist.len()
                                        ));
                                    }
                                    idxs.iter().zip(rlist.iter()).for_each(|(idx, val)| {
                                        if let Some(idx) = as_t!(idx, FInt) {
                                            let idx = idx.0 as usize;
                                            list[idx] = val.clone();
                                        } else {
                                            self.runtime_err(format!(
                                                "expected int but got {}",
                                                attr.type_str()
                                            ));
                                        }
                                    });
                                } else {
                                    self.runtime_err(format!(
                                        "expected type list but got {}",
                                        val.type_str()
                                    ));
                                }
                            } else {
                                self.runtime_err("indexes must not be empty".to_string());
                            }
                        } else {
                            self.runtime_err(format!(
                                "expected type list but got {}",
                                attr.type_str()
                            ));
                        }
                    } else if let Some(fobj) = as_t!(inst, FObj) {
                        let (obj, mutable) = unsafe { fobj.inner_mut().as_mut().unwrap() };

                        // checking for mutability
                        if !*mutable {
                            self.runtime_err("value is immutable".to_string());
                        }

                        let key = force_as_t!(attr, FVar);
                        obj.insert(key.0.clone(), val.clone());
                    } else if let Some(fstr) = as_t!(inst, FStr) {
                        // TODO: implement this
                    } else if as_t!(inst, FTup).is_some() {
                        self.runtime_err("tuple is immutable".to_string());
                    } else {
                        self.runtime_err(format!(
                            "expected type obj, list, or str but got {}",
                            inst.type_str()
                        ));
                    }

                    self.push(val);
                }

                OpCode::SetFnAddr => {
                    let func_obj = self.pop();
                    // getting the pointer where the function's body starts
                    let func_start = unsafe { current_frame!(self).ip.add(6) };
                    // getting the function object pointer
                    let func_ptr = force_as_t!(func_obj, FFunc).inner_mut();

                    // setting where the function starts
                    unsafe { (*func_ptr).set_addr(func_start) };

                    // re-pushing the modified function onto the stack
                    self.push(func_obj);

                    // don't need to do anything here because the LongJump instruction will skip
                    // the body of the function
                }

                OpCode::CallFn => {
                    // get the length of the function
                    let arg_len = read_byte!(self) as usize;

                    // getting the arguments to the function
                    let mut args = (0..arg_len).map(|_| self.pop()).collect::<Vec<Value>>();
                    args.reverse();

                    // hopefully a function
                    let func = self.pop();

                    if let Some(func) = as_t!(func, FFunc) {
                        // normal function
                        let func = unsafe { *func.inner_mut() };

                        if func.params > arg_len {
                            (0..(func.params - arg_len)).for_each(|_| args.push(FNil::build()));
                        }

                        if func.params == arg_len && func.rest {
                            args.push(FNil::build());
                        } else if (func.params == arg_len && !func.rest)
                            || (func.params < arg_len && func.rest)
                        {
                            {} // do nothing
                        } else {
                            self.runtime_err(format!(
                                "expected {} arguments but got {}",
                                func.params, arg_len
                            ));
                        }

                        // actually calling the function
                        self.call(func, args);
                    } else if let Some(nfunc) = as_t!(func, FNative) {
                        // native function
                        let func = nfunc.0;

                        // TODO: what path index should i pass here?
                        self.add_frame(Function::new(0, false, usize::MAX), std::ptr::null());

                        // calling the native function
                        func(self, args);

                        // removing the call frame
                        self.frames.pop();
                    } else {
                        self.runtime_err(format!(
                            "expected a function but got {}",
                            func.type_str()
                        ));
                    }
                }

                OpCode::RetFn => {
                    #[cfg(feature = "debug")]
                    {
                        let diff = self.frames[self.frames.len() - 2].ip as usize
                            - current_frame!(self).ip as usize;

                        self.debugger.offset += diff;
                    }

                    // resetting the call frame
                    self.frames.pop();
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
        current_frame!(self).ip = unsafe { current_frame!(self).ip.add(jmp) };
    }

    /// A short cut for random access stack assignment
    fn slot_assign(&mut self, idx: usize, val: Value) {
        slot_at_index!(self, idx) = val;
    }

    /// Defines or sets global or local variables
    fn define_or_set(&mut self, func: DefSetFunc, mutability: bool) {
        let right = self.pop();
        let left = self.pop();
        if let Some(var) = as_t!(left, FVar) {
            func(self, var.0.clone(), right.clone(), mutability);
        } else if let Some(left) = as_t!(left, FTup) {
            if let Some(right) = as_t!(right, FTup) {
                if right.0.len() != left.0.len() {
                    self.runtime_err(format!(
                        "invalid length: {} and {}",
                        left.0.len(),
                        right.0.len()
                    ));
                } else {
                    left.0.iter().zip(right.0.iter()).for_each(|(l, r)| {
                        if let Some(v) = as_t!(l, FVar) {
                            func(self, v.0.clone(), r.clone(), mutability);
                        } else if as_t!(l, FEmpty).is_some() {
                            {} // do nothing
                        } else {
                            self.runtime_err(format!(
                                "expected left side to be variable but got {}",
                                l.type_str()
                            ));
                        }
                    });
                }
            } else {
                self.runtime_err(format!("expected tup but got {}", right.type_str()));
            }
        } else if let Some(left) = as_t!(left, FList) {
            if let Some(right) = as_t!(right, FList) {
                if right.inner().0.len() != left.inner().0.len() {
                    self.runtime_err(format!(
                        "invalid length: {} and {}",
                        left.inner().0.len(),
                        right.inner().0.len()
                    ));
                } else {
                    left.inner()
                        .0
                        .iter()
                        .zip(right.inner().0.iter())
                        .for_each(|(l, r)| {
                            if let Some(v) = as_t!(l, FVar) {
                                func(self, v.0.clone(), r.clone(), mutability);
                            } else if as_t!(l, FEmpty).is_some() {
                                {} // do nothing
                            } else {
                                self.runtime_err(format!(
                                    "expected left side to be variable but got {}",
                                    l.type_str()
                                ));
                            }
                        });
                }
            } else {
                self.runtime_err(format!("expected list but got {}", left.type_str()));
            }
        } else if let Some(left) = as_t!(left, FObj) {
            if let Some(right) = as_t!(right, FObj) {
                if right.inner().0.len() != left.inner().0.len() {
                    self.runtime_err(format!(
                        "invalid length: {} and {}",
                        left.inner().0.len(),
                        right.inner().0.len()
                    ));
                } else {
                    left.inner().0.iter().for_each(|(key, assignee)| {
                        if let Some(val) = right.inner().0.get(key) {
                            if let Some(var) = as_t!(assignee, FVar) {
                                func(self, var.0.clone(), val.clone(), mutability);
                            } else if as_t!(assignee, FEmpty).is_some() {
                                {} // do nothing
                            } else {
                                self.runtime_err(format!(
                                    "expected left side to be variable but got {}",
                                    assignee.type_str()
                                ));
                            }
                        }
                    });
                }
            } else {
                self.runtime_err(format!("expected obj but got {}", right.type_str()));
            }
        } else {
            self.runtime_err(format!(
                "expected either list, tup, or obj but got {}",
                left.type_str()
            ));
        }

        self.push(right);
    }

    /// Calls a Flan function
    fn call(&mut self, func: Function, args: Vec<Value>) {
        self.add_frame(func, unsafe { func.addr.sub(1) });

        #[cfg(feature = "debug")]
        {
            let diff =
                self.frames[self.frames.len() - 2].ip as usize - current_frame!(self).ip as usize;

            self.debugger.offset -= diff;
        }

        // positional arguments
        let pos_args = &args[0..func.params];
        // rest argument
        let rest_args = if func.params != args.len() {
            Some(&args[func.params..])
        } else {
            None
        };

        // pushing the arguments onto the stack
        pos_args.iter().for_each(|arg| self.slots_push(arg.clone()));

        // if there's rest parameter, push the rest of the arguments as a list
        if let Some(rest_args) = rest_args {
            let rest_param = FList::build(self.heap, rest_args.to_vec(), false);
            self.slots_push(rest_param);
        }
    }

    /// Binds the given value to a global variable name
    fn define_global(vm: &mut VM, name: Arc<str>, val: Value, mutable: bool) {
        if let Entry::Vacant(e) = vm.globals.entry(name.clone()) {
            e.insert((val, mutable));
        } else {
            vm.runtime_err(format!("global variable {} is already defined", name));
        }
    }

    /// Rebinds a new value to a global variable
    fn set_global(vm: &mut VM, name: Arc<str>, val: Value, _: bool) {
        if vm.globals.get(&name).is_none() {
            vm.runtime_err(format!("global variable {} is not defined", &name));
        } else {
            let mutable = vm.globals[&name].1;
            if !mutable {
                vm.runtime_err(format!("global variable {} cannot be reassigned", &name));
            }
            vm.globals.insert(name, (val, mutable));
        }
    }

    /// Returns a Value from `constants`
    fn read_const(&mut self, long: bool) -> Value {
        let idx = if long {
            read_2bytes!(self) as usize
        } else {
            read_byte!(self) as usize
        };

        replace(&mut self.constants[idx], FNil::build())
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
    }

    fn slots_push(&mut self, val: Value) {
        // placeholder
        self.stack.push(FNil::build());

        current_frame_slot!(self) = val;
        current_frame!(self).slot_count += 1;
    }

    /// Pushes a new call frame to the frames array
    fn add_frame(&mut self, func: Function, addr: *const u8) {
        // checking for stack overflow
        if self.frames.len() == FRAME_MAX {
            self.runtime_err("stack overflow".to_string());
        }

        // creating a new call frame for the function call
        let frame = CallFrame {
            func,
            ip: addr,
            slot_bottom: self.stack.len(),
            slot_count: 0,
        };

        // setting the newly created call frame as the current frame
        self.frames.push(frame);
    }

    fn runtime_err(&mut self, msg: String) {
        let mut offset = unsafe { current_frame!(self).ip.offset_from(self.first_ip) } as usize;
        let mut pos = self.positions.get(&offset).unwrap();
        let mut node = Node {
            pos: *pos,
            path_idx: current_frame!(self).func.path_idx,
        };
        let mut stack = Stack::new_from_node(ErrType::Runtime, msg, node);
        while self.frames.len() != 1 {
            self.frames.pop();
            offset = unsafe { current_frame!(self).ip.offset_from(self.first_ip) } as usize;
            pos = self.positions.get(&offset).unwrap();
            node = Node {
                pos: *pos,
                path_idx: current_frame!(self).func.path_idx,
            };
            stack.add_node(node);
        }
        stack.report(1);
    }
}

pub fn test_execute(src: &str) {
    let (mem_slice, mut heap) = test_compile(src);

    Debug::run("TEST!", &mem_slice);

    VM::execute(mem_slice, &mut heap);

    heap.deallocate_all();
}
