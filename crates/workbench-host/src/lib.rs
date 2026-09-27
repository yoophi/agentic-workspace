//! Workbench 조립과 서버 생명주기(044, research R1). 데스크톱과 무관한 composition을 한 곳에 둔다:
//! 독립 서버(`apps/agentic-workbench-server`)와 AW embedded 모드가 같은 조립을 쓴다.

pub mod assembly;
pub mod http;
pub mod launch;
pub mod lifecycle;
pub mod mcp;
