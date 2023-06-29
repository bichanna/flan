use std::fmt;
use std::fs::File;
use std::io::Read;
use std::process::exit;
use std::sync::Arc;

/// A set of source file names (paths)
static mut PATHS: Vec<Arc<str>> = vec![];

/// A tuple containing a column and line numbers: (column, line)
pub type Position = (usize, usize);

/// Error types
#[derive(Debug)]
pub enum ErrType {
    Syntax,
}

/// The error stack
#[derive(Debug)]
pub struct Stack {
    /// The type of the error
    pub err: ErrType,
    /// The message
    pub msg: String,
    /// The error stack
    pub stack: Vec<Node>,
}

/// Prints out the given message and exits with the given exit code
pub fn flan_panic_exit(msg: &str, code: i32) {
    println!("FLAN PANIC: {}", msg);
    exit(code);
}

/// Node on the stack
#[derive(Debug)]
pub struct Node {
    /// The position of where the function is called
    pos: Position,
    /// The path index of the place where the error occurred
    path_idx: usize,
}

impl Stack {
    pub fn new_from_node(err: ErrType, msg: String, node: Node) -> Self {
        Self {
            err,
            msg,
            stack: vec![node],
        }
    }

    pub fn new(err: ErrType, msg: String, pos: Position, path_idx: usize) -> Self {
        Self::new_from_node(err, msg, Node { pos, path_idx })
    }

    pub fn add_path(filename: &str) {
        unsafe { PATHS.push(Arc::from(filename)) }
    }

    pub fn last_path_index() -> usize {
        unsafe { PATHS.len() - 1 }
    }
}

impl fmt::Display for Stack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut to_be_written = self.stack[1..]
            .iter()
            .rev()
            .map(|n| n.to_string())
            .collect::<Vec<String>>()
            .join("\n\n");

        to_be_written.push_str("\n\n");
        to_be_written.push_str(&self.stack[0].to_string());
        to_be_written.push_str(&format!("\n{:?}: {}\n", self.err, self.msg));

        f.write_str(&to_be_written)
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = unsafe { PATHS[self.path_idx].as_ref() };
        let file = File::open(path);
        if file.is_err() {
            flan_panic_exit(&format!("could not open {}", path), 1);
        }

        let mut contents = String::new();
        if file.unwrap().read_to_string(&mut contents).is_err() {
            flan_panic_exit(&format!("could not read {}", path), 1);
        }

        let lines = contents
            .split('\n')
            .map(Arc::from)
            .collect::<Vec<Arc<str>>>();
        let line = lines.get(self.pos.1);
        if line.is_none() {
            flan_panic_exit(&format!("invalid line number {}", self.pos.1), 1)
        }

        let mut err_msg = format!(
            "{}:{}:{}\n{}\n",
            path,
            self.pos.0,
            self.pos.1,
            line.unwrap()
        );

        err_msg.push_str(
            &(0..=self.pos.0)
                .map(|_| ' ')
                .collect::<Vec<char>>()
                .iter()
                .collect::<String>(),
        );

        err_msg.push('^');

        f.write_str(&err_msg)
    }
}
