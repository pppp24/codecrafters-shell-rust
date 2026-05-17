pub struct SimpleCommand {
    pub argv: Vec<String>,
    pub redirs: Vec<Redirection>,
}

pub struct Redirection {
    pub fd: u32, // 0 = stdin; 1 = stdout (default for '>'); 2 = stderr
    pub op: RedirOp,
    pub target: String,
}

#[derive(Debug, PartialEq)]
pub enum RedirOp {
    Out,
    Append,
}
