//! 서버 생명주기(044 research R4–R6, contracts/server-lifecycle.md): 단일 writer 잠금, 원자적 안내 파일, 소유자 신원·
//! 자격 증명, 신원 증명을 거친 클라이언트 확인, 시작 절차(`ensure`), 독립 서버 실행(`serve`)과 정지 감시(`monitor`).

pub mod calls;
pub mod client;
pub mod descriptor;
pub mod ensure;
pub mod identity;
pub mod lock;
pub mod monitor;
pub mod server;
pub mod stop;
