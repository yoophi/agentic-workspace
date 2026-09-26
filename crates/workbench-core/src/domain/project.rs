//! 프로젝트 도메인 모델. `apps/agentic-workbench/src-tauri/src/domain/project.rs`에서 이동했다(037).
//! 직렬화 형태(camelCase 4필드)는 프론트 `entities/project/model/types.ts`와 계약이므로 바꾸지 않는다.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub working_directory: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDraft {
    pub name: String,
    pub working_directory: String,
    pub description: Option<String>,
}
