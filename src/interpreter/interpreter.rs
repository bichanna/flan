use super::engine::Engine;
use super::scope::Scope;
use std::collections::HashMap;

pub struct Context<'a> {
    // shared interpreter state
    pub engine: &'a mut Engine<'a>,
    // directory containing the root file of this context, used for loading other modules with
    // relative paths
    pub root_path: &'a str,
    // top level global scope of this context
    pub scope: Scope<'a>,
}

impl<'a> Context<'a> {
    pub fn new(root_path: &'a str, engine: &'a mut Engine<'a>) -> Self {
        Self {
            root_path,
            engine,
            scope: Scope {
                parent: None,
                vars: HashMap::new(),
            },
        }
    }

    pub fn child_context(&mut self, root_path: &'a str, engine: &'a mut Engine<'a>) -> Self {
        Self {
            engine,
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
