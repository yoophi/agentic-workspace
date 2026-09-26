//! 호출자 정체. wire에 실리지 않으며 인증 계층만 생성한다(정본 Invariant 1).

use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 호출자 종류. 037에는 데스크톱과 테스트용만 있다. 3단계에서 human CLI·agent 등이 추가된다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PrincipalKind {
    Desktop,
    Test,
}

impl PrincipalKind {
    /// ledger `principal_kind` 컬럼 값.
    pub fn as_str(self) -> &'static str {
        match self {
            PrincipalKind::Desktop => "desktop",
            PrincipalKind::Test => "test",
        }
    }
}

impl fmt::Display for PrincipalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 권한 범위. descriptor의 `requiredScopes`로 wire에 노출되므로 serde를 가진다.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema,
)]
pub enum Scope {
    #[serde(rename = "project:read")]
    ProjectRead,
    #[serde(rename = "project:write")]
    ProjectWrite,
    #[serde(rename = "system:describe")]
    SystemDescribe,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::ProjectRead => "project:read",
            Scope::ProjectWrite => "project:write",
            Scope::SystemDescribe => "system:describe",
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 인증을 통과한 호출자. `Serialize`를 의도적으로 구현하지 않는다 — 입력으로 정체를 지정할 수 없어야 한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPrincipal {
    pub kind: PrincipalKind,
    pub scopes: BTreeSet<Scope>,
}

impl AuthenticatedPrincipal {
    pub fn new(kind: PrincipalKind, scopes: impl IntoIterator<Item = Scope>) -> Self {
        Self {
            kind,
            scopes: scopes.into_iter().collect(),
        }
    }

    /// 데스크톱 앱 조립부가 Tauri compat Adapter에 고정 주입하는 전체 권한 호출자.
    pub fn desktop() -> Self {
        Self::new(
            PrincipalKind::Desktop,
            [
                Scope::ProjectRead,
                Scope::ProjectWrite,
                Scope::SystemDescribe,
            ],
        )
    }

    /// 테스트용 조회 전용 호출자.
    pub fn test_readonly() -> Self {
        Self::new(
            PrincipalKind::Test,
            [Scope::ProjectRead, Scope::SystemDescribe],
        )
    }

    pub fn has_scope(&self, scope: Scope) -> bool {
        self.scopes.contains(&scope)
    }

    pub fn has_all(&self, scopes: &[Scope]) -> bool {
        scopes.iter().all(|scope| self.has_scope(*scope))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_has_every_scope_and_readonly_lacks_write() {
        let desktop = AuthenticatedPrincipal::desktop();
        assert!(desktop.has_all(&[
            Scope::ProjectRead,
            Scope::ProjectWrite,
            Scope::SystemDescribe
        ]));
        let readonly = AuthenticatedPrincipal::test_readonly();
        assert!(readonly.has_scope(Scope::ProjectRead));
        assert!(readonly.has_scope(Scope::SystemDescribe));
        assert!(!readonly.has_scope(Scope::ProjectWrite));
        assert_eq!(readonly.kind.as_str(), "test");
    }

    #[test]
    fn scope_wire_names_use_colon_form() {
        assert_eq!(
            serde_json::to_value(Scope::ProjectWrite).unwrap(),
            "project:write"
        );
        assert_eq!(Scope::SystemDescribe.to_string(), "system:describe");
    }
}
