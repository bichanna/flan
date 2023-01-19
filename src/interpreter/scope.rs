use super::value::Value;
use std::collections::HashMap;

pub struct Scope<'a> {
    pub parent: Option<Box<Scope<'a>>>,
    pub vars: HashMap<&'a str, Value>,
}

impl<'a> Scope<'a> {
    pub fn get(&self, name: &str) -> Result<&Value, String> {
        if let Some(v) = self.vars.get(name) {
            Ok(v)
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            Err(format!("{} is undefined", name))
        }
    }

    pub fn put(&mut self, name: &'a str, v: Value) {
        if self.vars.contains_key(name) {
            self.vars.remove(name);
        }
        self.vars.insert(name, v);
    }

    pub fn update(&mut self, name: &'a str, v: Value) -> Option<String> {
        if self.vars.contains_key(name) {
            self.vars.remove(name);
            self.vars.insert(name, v);
            None
        } else {
            if let Some(parent) = &self.parent {
                parent.update(name, v)
            } else {
                Some(format!("{} is undefined", name))
            }
        }
    }
}
