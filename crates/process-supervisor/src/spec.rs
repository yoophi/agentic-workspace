use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessOwner {
    pub owner_id: String,
    pub attempt_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamPolicy {
    ProtocolFrames { max_frame_bytes: usize },
    ParsedCapture { max_bytes: usize },
    DisplayLog { max_bytes: usize, max_events: usize },
    Null,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminationPolicy {
    pub graceful_timeout_ms: u64,
    pub force_timeout_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessSpec {
    pub owner: ProcessOwner,
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub stdout: StreamPolicy,
    pub stderr: StreamPolicy,
    pub termination: TerminationPolicy,
}
