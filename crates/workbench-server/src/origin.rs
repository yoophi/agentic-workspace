//! 출처·Host 정책(042 research R7, contracts §1). 비교는 **문자열 전체 일치**뿐이다 — 접두사·접미사 비교는
//! `http://127.0.0.1.evil.example` 같은 출처를 통과시킨다(AW MCP 서버의 옛 결함). Origin은 인증 수단이 아니다:
//! 없으면(비브라우저) 판정을 자격 증명에 맡긴다.

/// 요청 Origin 판정.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginCheck {
    /// Origin 헤더 없음(비브라우저 클라이언트).
    Absent,
    Allowed,
    Rejected,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OriginPolicy {
    allowed: Vec<String>,
}

impl OriginPolicy {
    pub fn new(allowed: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            allowed: allowed.into_iter().map(Into::into).collect(),
        }
    }

    pub fn allowed(&self) -> &[String] {
        &self.allowed
    }

    /// `null`·빈 값은 거절한다(여러 opaque origin을 구분할 수 없다).
    pub fn check(&self, origin: Option<&str>) -> OriginCheck {
        match origin {
            None => OriginCheck::Absent,
            Some(value) if value.is_empty() || value == "null" => OriginCheck::Rejected,
            Some(value) if self.allowed.iter().any(|allowed| allowed == value) => {
                OriginCheck::Allowed
            }
            Some(_) => OriginCheck::Rejected,
        }
    }
}

/// Host 헤더 정책: 루프백 bind 포트에 대해 `127.0.0.1:<port>`·`localhost:<port>`만(DNS rebinding 방지).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostPolicy {
    port: u16,
}

impl HostPolicy {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub fn allows(&self, host: Option<&str>) -> bool {
        let Some(host) = host else {
            return false;
        };
        host == format!("127.0.0.1:{}", self.port) || host == format!("localhost:{}", self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> OriginPolicy {
        OriginPolicy::new(["http://localhost:1420", "tauri://localhost"])
    }

    #[test]
    fn origins_match_exactly() {
        let policy = policy();
        assert_eq!(policy.check(None), OriginCheck::Absent);
        assert_eq!(
            policy.check(Some("http://localhost:1420")),
            OriginCheck::Allowed
        );
        assert_eq!(
            policy.check(Some("tauri://localhost")),
            OriginCheck::Allowed
        );
        for rejected in [
            "http://localhost:14200",
            "http://localhost:1420.evil.example",
            "http://localhost",
            "HTTP://LOCALHOST:1420",
            "http://127.0.0.1.evil.example",
            "tauri://localhost.evil",
            "null",
            "",
        ] {
            assert_eq!(
                policy.check(Some(rejected)),
                OriginCheck::Rejected,
                "{rejected}"
            );
        }
    }

    #[test]
    fn host_must_be_the_bound_loopback_port() {
        let host = HostPolicy::new(4000);
        assert!(host.allows(Some("127.0.0.1:4000")));
        assert!(host.allows(Some("localhost:4000")));
        for rejected in [
            None,
            Some("127.0.0.1"),
            Some("127.0.0.1:4001"),
            Some("evil.example:4000"),
            Some("127.0.0.1:4000.evil.example"),
            Some("0.0.0.0:4000"),
        ] {
            assert!(!host.allows(rejected), "{rejected:?}");
        }
    }
}
