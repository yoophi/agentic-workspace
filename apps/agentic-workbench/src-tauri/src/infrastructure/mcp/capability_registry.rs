//! Run-scoped MCP capability registry. 041: 토큰은 **run 하나**만 가리킨다(research R7) — orchestration 역할은
//! 토큰 주장이 아니라 서버 상태(`orchestration.getAgentRole`)로 정한다. 재시도·재배정·교대로 물러난 run의 토큰은
//! core가 `RunLaunchDecorator::revoke_run`으로 폐기한다(토큰 수명 관리, 권한 근거는 아니다).

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use uuid::Uuid;

/// 토큰이 인증한 주체: MCP 실행 토큰이 묶인 run.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CapabilityPrincipal {
    pub run_id: String,
}

impl CapabilityPrincipal {
    pub fn run(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
        }
    }
}

#[derive(Clone, Default)]
pub struct CapabilityRegistry {
    entries: Arc<RwLock<HashMap<String, CapabilityPrincipal>>>,
}

fn write(
    registry: &CapabilityRegistry,
) -> std::sync::RwLockWriteGuard<'_, HashMap<String, CapabilityPrincipal>> {
    registry
        .entries
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl CapabilityRegistry {
    pub fn issue(&self, run_id: &str) -> String {
        let token = format!("awcap_{}", Uuid::new_v4().simple());
        write(self).insert(token.clone(), CapabilityPrincipal::run(run_id));
        token
    }

    /// 모르는·폐기된 토큰이면 `None`(호출자가 "MCP capability is invalid or expired."로 답한다).
    pub fn resolve(&self, token: &str) -> Option<CapabilityPrincipal> {
        self.entries
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(token)
            .cloned()
    }

    pub fn revoke_run(&self, run_id: &str) {
        write(self).retain(|_, principal| principal.run_id != run_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_resolve_to_their_run_only() {
        let registry = CapabilityRegistry::default();
        let token = registry.issue("run-1");
        assert_ne!(token, "run-1");
        assert_eq!(
            registry.resolve(&token),
            Some(CapabilityPrincipal::run("run-1"))
        );
        assert!(registry.resolve("run-1").is_none());
    }

    #[test]
    fn revoking_a_run_drops_only_its_tokens() {
        let registry = CapabilityRegistry::default();
        let old = registry.issue("run-old");
        let current = registry.issue("run-current");
        registry.revoke_run("run-old");
        assert!(registry.resolve(&old).is_none());
        assert!(registry.resolve(&current).is_some());
    }
}
