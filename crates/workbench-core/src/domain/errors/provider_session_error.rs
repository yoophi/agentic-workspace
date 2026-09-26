//! provider 세션 조회 오류. 손상된 세션 파일은 오류가 아니라 건너뛴다 — 여기 오는 것은 루트 경로를 정할 수 없는 등
//! 목록 전체를 만들 수 없는 경우뿐이다. 문구는 오늘 command가 돌려주던 `anyhow` 문구와 같다.

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderSessionError {
    #[error("{0}")]
    Storage(String),
}
