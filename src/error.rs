#[derive(Clone, Debug, PartialEq)]
pub struct ParserError {
    msg: String,
    pub line: usize,
    pub col: usize,
}

impl ParserError {
    pub fn new(msg: &str, line: usize, col: usize) -> Self {
        ParserError {
            msg: String::from(msg),
            line,
            col,
        }
    }

    pub fn format(&self, filename: &str) -> String {
        format!(
            "{}:{}:{} error: {}",
            filename, self.line, self.col, self.msg
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StackEntry<'a> {
    name: Option<&'a str>,
    line: usize,
    col: usize,
}

impl<'a> StackEntry<'a> {
    pub fn new<'b>(name: Option<&'a str>, line: usize, col: usize) -> Self {
        Self { name, line, col }
    }

    pub fn string(&self) -> String {
        if let Some(name) = self.name {
            format!("  {}:{}:in func {}", self.line, self.col, name)
        } else {
            format!("  {}:{}:in anonymous func", self.line, self.col)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeError<'a> {
    reason: &'a str,
    line: usize,
    col: usize,
    stack_traces: Vec<StackEntry<'a>>,
}

impl<'a> RuntimeError<'a> {
    pub fn new<'b>(
        reason: &'a str,
        line: usize,
        col: usize,
        stack_traces: Option<Vec<StackEntry<'a>>>,
    ) -> Self {
        Self {
            reason,
            line,
            col,
            stack_traces: if let Some(stack_traces) = stack_traces {
                stack_traces
            } else {
                vec![]
            },
        }
    }

    pub fn string(&self, filename: &str) -> String {
        format!(
            "{}\n{}:{}:{} error: {}",
            self.stack_traces
                .to_owned()
                .into_iter()
                .map(|s| s.string())
                .collect::<Vec<String>>()
                .join("\n"),
            filename,
            self.line,
            self.col,
            self.reason
        )
    }
}
