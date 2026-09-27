//! 호출자 정체. wire에 실리지 않으며 인증 계층만 생성한다(정본 Invariant 1).

use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 호출자 종류. 040에서 MCP 실행 토큰으로 인증된 agent가 추가됐다(ADR 0006). 3단계에서 human CLI 등이 추가된다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PrincipalKind {
    Desktop,
    Test,
    Agent,
    /// 044: 서버 안내 파일의 소유자 자격 증명을 가진 클라이언트(`local:owner`). 모든 작업대를 다루고 서버를 정지할 수 있다.
    Owner,
}

impl PrincipalKind {
    /// ledger `principal_kind` 컬럼 값.
    pub fn as_str(self) -> &'static str {
        match self {
            PrincipalKind::Desktop => "desktop",
            PrincipalKind::Test => "test",
            PrincipalKind::Agent => "agent",
            PrincipalKind::Owner => "owner",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "desktop" => Some(PrincipalKind::Desktop),
            "test" => Some(PrincipalKind::Test),
            "agent" => Some(PrincipalKind::Agent),
            "owner" => Some(PrincipalKind::Owner),
            _ => None,
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
    #[serde(rename = "savedPrompt:read")]
    SavedPromptRead,
    #[serde(rename = "savedPrompt:write")]
    SavedPromptWrite,
    #[serde(rename = "goal:read")]
    GoalRead,
    #[serde(rename = "goal:write")]
    GoalWrite,
    #[serde(rename = "agentRunSettings:read")]
    AgentRunSettingsRead,
    #[serde(rename = "agentRunSettings:write")]
    AgentRunSettingsWrite,
    #[serde(rename = "git:read")]
    GitRead,
    #[serde(rename = "git:write")]
    GitWrite,
    #[serde(rename = "worktree:read")]
    WorktreeRead,
    #[serde(rename = "agent:read")]
    AgentRead,
    /// run 이벤트 스트림 구독(039).
    #[serde(rename = "run:read")]
    RunRead,
    /// run 시작·제어(040).
    #[serde(rename = "run:write")]
    RunWrite,
    /// 작업대 알림 스트림 구독(040).
    #[serde(rename = "bench:read")]
    BenchRead,
    /// 작업대 열기·닫기(040).
    #[serde(rename = "bench:write")]
    BenchWrite,
    /// 교환 조회·교환 스트림 구독(040).
    #[serde(rename = "exchange:read")]
    ExchangeRead,
    /// 교환 동기화·전송·확인(040).
    #[serde(rename = "exchange:write")]
    ExchangeWrite,
    /// 창 제목 같은 표현 요청(040, ADR 0007).
    #[serde(rename = "presentation:write")]
    PresentationWrite,
    /// 041: orchestration 작업 영역 조회·스트림.
    #[serde(rename = "orchestration:read")]
    OrchestrationRead,
    /// 041: orchestration 작업 영역 변경(데스크톱 동작·agent 도구).
    #[serde(rename = "orchestration:write")]
    OrchestrationWrite,
    #[serde(rename = "system:describe")]
    SystemDescribe,
    /// 044: 서버 상태 조회. 소유자 주체만 갖는다.
    #[serde(rename = "server:read")]
    ServerRead,
    /// 044: 서버 정지·임대·창 토큰 발급·창 폐기. 소유자 주체만 갖는다.
    #[serde(rename = "server:admin")]
    ServerAdmin,
}

impl Scope {
    /// 전체 scope. 소유자(`owner()`)가 이 집합을 갖는다. 데스크톱·창·시험 주체는 소유자 전용 scope를 뺀
    /// [`desktop_scopes`]를 갖는다(044).
    pub const ALL: [Scope; 24] = [
        Scope::ProjectRead,
        Scope::ProjectWrite,
        Scope::SavedPromptRead,
        Scope::SavedPromptWrite,
        Scope::GoalRead,
        Scope::GoalWrite,
        Scope::AgentRunSettingsRead,
        Scope::AgentRunSettingsWrite,
        Scope::GitRead,
        Scope::GitWrite,
        Scope::WorktreeRead,
        Scope::AgentRead,
        Scope::RunRead,
        Scope::RunWrite,
        Scope::BenchRead,
        Scope::BenchWrite,
        Scope::ExchangeRead,
        Scope::ExchangeWrite,
        Scope::PresentationWrite,
        Scope::OrchestrationRead,
        Scope::OrchestrationWrite,
        Scope::SystemDescribe,
        Scope::ServerRead,
        Scope::ServerAdmin,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Scope::ProjectRead => "project:read",
            Scope::ProjectWrite => "project:write",
            Scope::SavedPromptRead => "savedPrompt:read",
            Scope::SavedPromptWrite => "savedPrompt:write",
            Scope::GoalRead => "goal:read",
            Scope::GoalWrite => "goal:write",
            Scope::AgentRunSettingsRead => "agentRunSettings:read",
            Scope::AgentRunSettingsWrite => "agentRunSettings:write",
            Scope::GitRead => "git:read",
            Scope::GitWrite => "git:write",
            Scope::WorktreeRead => "worktree:read",
            Scope::AgentRead => "agent:read",
            Scope::RunRead => "run:read",
            Scope::RunWrite => "run:write",
            Scope::BenchRead => "bench:read",
            Scope::BenchWrite => "bench:write",
            Scope::ExchangeRead => "exchange:read",
            Scope::ExchangeWrite => "exchange:write",
            Scope::PresentationWrite => "presentation:write",
            Scope::OrchestrationRead => "orchestration:read",
            Scope::OrchestrationWrite => "orchestration:write",
            Scope::SystemDescribe => "system:describe",
            Scope::ServerRead => "server:read",
            Scope::ServerAdmin => "server:admin",
        }
    }

    /// 조회 scope인지(`:read` 또는 `system:describe`).
    pub fn is_read(self) -> bool {
        !matches!(
            self,
            Scope::ProjectWrite
                | Scope::SavedPromptWrite
                | Scope::GoalWrite
                | Scope::AgentRunSettingsWrite
                | Scope::GitWrite
                | Scope::RunWrite
                | Scope::BenchWrite
                | Scope::ExchangeWrite
                | Scope::PresentationWrite
                | Scope::OrchestrationWrite
                | Scope::ServerAdmin
        )
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 호출자 주체(040). 작업대는 연 주체에 묶인다(ADR core 0004). `desktop`, `desktop:window:<label>:<incarnation>`, `test:<name>`, `agent:<runId>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PrincipalSubject(String);

impl PrincipalSubject {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PrincipalSubject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

const AGENT_SUBJECT_PREFIX: &str = "agent:";
const DESKTOP_WINDOW_SUBJECT_PREFIX: &str = "desktop:window:";

/// 데스크톱·창·시험 주체의 scope: 전체에서 소유자 전용 `server:read`·`server:admin`을 뺀다(044). 창 토큰은 서버
/// 상태 조회·정지·토큰 발급을 할 수 없다.
pub fn desktop_scopes() -> impl Iterator<Item = Scope> {
    Scope::ALL
        .into_iter()
        .filter(|scope| !matches!(scope, Scope::ServerRead | Scope::ServerAdmin))
}

/// agent principal의 scope: 교환 조회·쓰기와 표현 요청만(ADR 0006).
pub const AGENT_SCOPES: [Scope; 6] = [
    Scope::ExchangeRead,
    Scope::ExchangeWrite,
    Scope::PresentationWrite,
    Scope::OrchestrationRead,
    Scope::OrchestrationWrite,
    Scope::SystemDescribe,
];

/// 인증을 통과한 호출자. `Serialize`를 의도적으로 구현하지 않는다 — 입력으로 정체를 지정할 수 없어야 한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPrincipal {
    pub kind: PrincipalKind,
    pub subject: PrincipalSubject,
    pub scopes: BTreeSet<Scope>,
}

impl AuthenticatedPrincipal {
    /// 주체는 종류 이름과 같다(`desktop`, `test`). 다른 주체가 필요하면 전용 생성자를 쓴다.
    pub fn new(kind: PrincipalKind, scopes: impl IntoIterator<Item = Scope>) -> Self {
        Self {
            kind,
            subject: PrincipalSubject::new(kind.as_str()),
            scopes: scopes.into_iter().collect(),
        }
    }

    /// 데스크톱 앱 조립부가 Tauri compat Adapter에 고정 주입하는 전체 권한 호출자.
    pub fn desktop() -> Self {
        Self::new(PrincipalKind::Desktop, desktop_scopes())
    }

    /// 044 소유자 주체(`local:owner`): 모든 scope. 작업대 소유 판정 우회는 kind로 판단한다(core).
    pub fn owner() -> Self {
        Self {
            subject: PrincipalSubject::new("local:owner"),
            ..Self::new(PrincipalKind::Owner, Scope::ALL)
        }
    }

    /// 데스크톱 창 하나(043). 주체 `desktop:window:<label>:<incarnation>` — 창마다 작업대 소유가 갈린다. incarnation은
    /// 창이 만들어질 때마다 새로 발급되므로, 같은 label로 다시 연 창은 이전 창의 작업대·토큰과 다른 주체다.
    pub fn desktop_window(label: &str, incarnation: &str) -> Self {
        Self {
            subject: PrincipalSubject::new(format!(
                "{DESKTOP_WINDOW_SUBJECT_PREFIX}{label}:{incarnation}"
            )),
            ..Self::desktop()
        }
    }

    /// 테스트용 조회 전용 호출자: 모든 `:read` + `system:describe`.
    pub fn test_readonly() -> Self {
        Self {
            subject: PrincipalSubject::new("test:readonly"),
            ..Self::new(
                PrincipalKind::Test,
                desktop_scopes().filter(|scope| scope.is_read()),
            )
        }
    }

    /// 테스트용 다른 주체: 데스크톱과 같은 scope, 주체 `test:<name>`(교차 주체 시나리오).
    pub fn test_as(name: &str) -> Self {
        Self {
            subject: PrincipalSubject::new(format!("test:{name}")),
            ..Self::new(PrincipalKind::Test, desktop_scopes())
        }
    }

    /// MCP 실행 토큰으로 인증된 agent(040). run 하나에 묶인다.
    pub fn agent(run_id: &str) -> Self {
        Self {
            subject: PrincipalSubject::new(format!("{AGENT_SUBJECT_PREFIX}{run_id}")),
            ..Self::new(PrincipalKind::Agent, AGENT_SCOPES)
        }
    }

    /// agent principal이면 묶인 run id.
    pub fn agent_run_id(&self) -> Option<&str> {
        if self.kind != PrincipalKind::Agent {
            return None;
        }
        self.subject.as_str().strip_prefix(AGENT_SUBJECT_PREFIX)
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
    fn desktop_has_all_22_and_readonly_has_only_reads() {
        let desktop = AuthenticatedPrincipal::desktop();
        assert_eq!(desktop.scopes.len(), 22);
        let readonly = AuthenticatedPrincipal::test_readonly();
        assert_eq!(
            readonly.scopes.len(),
            12,
            "read 11(run·bench·exchange·orchestration 포함) + system:describe"
        );
        for scope in Scope::ALL {
            // 044: 소유자 전용 scope는 데스크톱·창·시험 주체에게 없다(조회 scope인 `server:read` 포함).
            let owner_only = matches!(scope, Scope::ServerRead | Scope::ServerAdmin);
            assert_eq!(readonly.has_scope(scope), scope.is_read() && !owner_only, "{scope}");
            assert_eq!(desktop.has_scope(scope), !owner_only, "{scope}");
            assert_eq!(serde_json::to_value(scope).unwrap(), scope.as_str());
        }
        let owner = AuthenticatedPrincipal::owner();
        assert_eq!(owner.scopes.len(), Scope::ALL.len());
        assert_eq!(owner.subject.as_str(), "local:owner");
        assert!(!AuthenticatedPrincipal::desktop_window("w", "i").has_scope(Scope::ServerAdmin));
        assert!(!readonly.has_scope(Scope::GitWrite));
        assert!(readonly.has_scope(Scope::WorktreeRead));
        assert!(readonly.has_scope(Scope::RunRead));
    }

    #[test]
    fn scope_wire_names_use_colon_form() {
        assert_eq!(
            serde_json::to_value(Scope::ProjectWrite).unwrap(),
            "project:write"
        );
        assert_eq!(Scope::SystemDescribe.to_string(), "system:describe");
        assert_eq!(
            Scope::AgentRunSettingsWrite.to_string(),
            "agentRunSettings:write"
        );
    }

    #[test]
    fn subjects_distinguish_principals_and_agent_is_bound_to_its_run() {
        assert_eq!(
            AuthenticatedPrincipal::desktop().subject.as_str(),
            "desktop"
        );
        assert_eq!(
            AuthenticatedPrincipal::test_readonly().subject.as_str(),
            "test:readonly"
        );
        let other = AuthenticatedPrincipal::test_as("desktop2");
        assert_eq!(other.subject.as_str(), "test:desktop2");
        assert_eq!(other.scopes.len(), 22);

        let agent = AuthenticatedPrincipal::agent("r1");
        assert_eq!(agent.kind.as_str(), "agent");
        assert_eq!(agent.agent_run_id(), Some("r1"));
        assert_eq!(
            agent.scopes.iter().copied().collect::<Vec<_>>(),
            vec![
                Scope::ExchangeRead,
                Scope::ExchangeWrite,
                Scope::PresentationWrite,
                Scope::OrchestrationRead,
                Scope::OrchestrationWrite,
                Scope::SystemDescribe
            ]
        );
        assert_eq!(AuthenticatedPrincipal::desktop().agent_run_id(), None);
        assert_eq!(PrincipalKind::parse("agent"), Some(PrincipalKind::Agent));
    }
}
