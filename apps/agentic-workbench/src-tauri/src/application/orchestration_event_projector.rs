//! Projects runtime worker events into durable orchestration events.

use crate::domain::{
    agent_orchestration::{ExecutionStatus, TaskStatus},
    events::{LifecycleStatus, RunEvent},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RuntimeProjection {
    pub execution_status: ExecutionStatus,
    pub task_status: Option<TaskStatus>,
    pub attention: bool,
}

pub fn project_runtime_event(
    event: &RunEvent,
    has_explicit_result: bool,
    task_already_cancelled: bool,
) -> RuntimeProjection {
    match event {
        RunEvent::Lifecycle { status, .. } => match status {
            LifecycleStatus::Started
            | LifecycleStatus::Initialized
            | LifecycleStatus::SessionCreated
            | LifecycleStatus::PromptSent
            | LifecycleStatus::SteerPending
            | LifecycleStatus::SteerAccepted
            | LifecycleStatus::SteerRejected => RuntimeProjection {
                execution_status: ExecutionStatus::Active,
                task_status: None,
                attention: false,
            },
            LifecycleStatus::PromptCompleted => RuntimeProjection {
                execution_status: ExecutionStatus::Idle,
                task_status: None,
                attention: false,
            },
            LifecycleStatus::Cancelled => RuntimeProjection {
                execution_status: ExecutionStatus::Stopped,
                task_status: task_already_cancelled.then_some(TaskStatus::Cancelled),
                attention: false,
            },
            LifecycleStatus::Completed => RuntimeProjection {
                execution_status: ExecutionStatus::Stopped,
                task_status: Some(if has_explicit_result {
                    TaskStatus::Completed
                } else {
                    TaskStatus::Failed
                }),
                attention: !has_explicit_result,
            },
        },
        RunEvent::Error { .. } | RunEvent::Diagnostic { .. } => RuntimeProjection {
            execution_status: ExecutionStatus::Active,
            task_status: None,
            attention: true,
        },
        _ => RuntimeProjection {
            execution_status: ExecutionStatus::Active,
            task_status: None,
            attention: false,
        },
    }
}

pub fn project_worktree_guard(changed: bool) -> Option<RuntimeProjection> {
    changed.then_some(RuntimeProjection {
        execution_status: ExecutionStatus::Stopped,
        task_status: Some(TaskStatus::Failed),
        attention: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_completion_is_not_task_completion() {
        let projection = project_runtime_event(
            &RunEvent::Lifecycle {
                status: LifecycleStatus::PromptCompleted,
                message: "done".into(),
            },
            false,
            false,
        );
        assert_eq!(projection.execution_status, ExecutionStatus::Idle);
        assert_eq!(projection.task_status, None);
    }

    #[test]
    fn process_completion_without_result_fails_instead_of_completing() {
        let projection = project_runtime_event(
            &RunEvent::Lifecycle {
                status: LifecycleStatus::Completed,
                message: "exit".into(),
            },
            false,
            false,
        );
        assert_eq!(projection.task_status, Some(TaskStatus::Failed));
        assert!(projection.attention);
    }

    #[test]
    fn unexpected_worktree_change_fails_and_requires_attention() {
        let projection = project_worktree_guard(true).unwrap();
        assert_eq!(projection.task_status, Some(TaskStatus::Failed));
        assert!(projection.attention);
        assert!(project_worktree_guard(false).is_none());
    }
}
