use super::scope::Scope;
use std::collections::HashMap;
use std::fs::File;
use std::os::unix::io::RawFd;

#[derive(Debug)]
pub struct Engine<'a> {
    // for duplicating imports
    pub import_map: HashMap<&'a str, Scope<'a>>,
    // file fd -> fs::File map
    pub file_map: HashMap<RawFd, File>, // TODO: This probably only works for UNIX-like systems,
                                        // which excludes Windows, so do something!
}

impl<'a> Engine<'a> {
    pub fn new() -> Self {
        Self {
            import_map: HashMap::new(),
            file_map: HashMap::new(),
        }
    }
}
