//! 프로젝트 저장소 port. AW `domain/project_repository.rs`에서 이동하며 오류 타입과 복구 메서드를 더했다.

use crate::domain::{project::Project, project_error::ProjectError};

pub trait ProjectRepository: Send + Sync {
    /// 읽기 전용. 파일이 없으면 빈 벡터, 파싱 실패면 `ProjectError::StoreCorrupt`. 어떤 경우에도 쓰지 않는다.
    fn load_projects(&self) -> Result<Vec<Project>, ProjectError>;

    /// 원자적으로 저장한다(temp + rename, 이전 파일은 `.bak`).
    fn save_projects(&self, projects: &[Project]) -> Result<(), ProjectError>;

    /// 손상된 primary를 `.bak`으로 교체한다. **aggregate lock을 잡은 호출자만** 불러야 한다.
    /// primary가 이미 정상이면 아무 것도 하지 않는다.
    fn recover_from_backup(&self) -> Result<(), ProjectError>;
}
