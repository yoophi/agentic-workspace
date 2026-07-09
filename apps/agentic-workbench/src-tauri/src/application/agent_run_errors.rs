use std::fmt;

use crate::ports::session_registry::ReserveRunError;

#[derive(Debug, PartialEq, Eq)]
pub enum StartAgentRunError {
    ReserveRun(ReserveRunError),
    AttachRunHandle(String),
}

impl fmt::Display for StartAgentRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReserveRun(error) => error.fmt(f),
            Self::AttachRunHandle(message) => f.write_str(message),
        }
    }
}

impl From<StartAgentRunError> for String {
    fn from(error: StartAgentRunError) -> Self {
        error.to_string()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SendPromptError {
    EmptyPrompt,
    RunNotActive,
    DispatchFailed(String),
}

impl fmt::Display for SendPromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPrompt => f.write_str("prompt is empty"),
            Self::RunNotActive => f.write_str("agent run is not active"),
            Self::DispatchFailed(message) => write!(f, "prompt dispatch failed: {message}"),
        }
    }
}

impl From<SendPromptError> for String {
    fn from(error: SendPromptError) -> Self {
        error.to_string()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SteerPromptError {
    EmptyPrompt,
    RunNotActive,
    Unsupported(String),
    DispatchFailed(String),
}

impl fmt::Display for SteerPromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPrompt => f.write_str("steer prompt is empty"),
            Self::RunNotActive => f.write_str("agent run is not active"),
            Self::Unsupported(message) => write!(f, "steer unsupported: {message}"),
            Self::DispatchFailed(message) => write!(f, "steer dispatch failed: {message}"),
        }
    }
}

impl From<SteerPromptError> for String {
    fn from(error: SteerPromptError) -> Self {
        error.to_string()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SetPermissionModeError {
    RunNotActive,
    Apply(String),
}

impl fmt::Display for SetPermissionModeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RunNotActive => f.write_str("agent run is not active"),
            Self::Apply(message) => f.write_str(message),
        }
    }
}

impl From<SetPermissionModeError> for String {
    fn from(error: SetPermissionModeError) -> Self {
        error.to_string()
    }
}
