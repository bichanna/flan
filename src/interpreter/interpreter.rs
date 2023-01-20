use super::engine::Engine;
use super::scope::Scope;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Context<'a> {
    // shared interpreter state
    pub engine: Rc<RefCell<Engine<'a>>>,
    // directory containing the root file of this context, used for loading other modules with
    // relative paths
    pub root_path: &'a str,
    // top level global scope of this context
    pub scope: Scope<'a>,
}

impl<'a> Context<'a> {
    pub fn new(root_path: &'a str) -> Self {
        Self {
            root_path,
            engine: Rc::new(RefCell::new(Engine::new())),
            scope: Scope {
                parent: None,
                vars: HashMap::new(),
            },
        }
    }

    pub fn child_context(&mut self, root_path: &'a str) -> Self {
        Self {
            engine: self.engine.to_owned(),
            root_path,
            scope: Scope {
                parent: None,
                vars: HashMap::new(),
            },
        }
    }

    pub fn sub_scope(&mut self, parent: Scope<'a>) {
        self.scope = Scope {
            parent: Some(Box::new(parent)),
            vars: HashMap::new(),
        }
    }
}

// Actual interpreter implementation
impl<'a> Context<'a> {}
