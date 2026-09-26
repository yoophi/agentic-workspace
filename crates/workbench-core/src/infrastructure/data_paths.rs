//! 서버 런타임이 소유하는 데이터 경로. `Json*Repository::from_app(&AppHandle)`을 대체하는 생성자 주입 값이다.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPaths {
    app_data_dir: PathBuf,
}

impl DataPaths {
    pub fn new(app_data_dir: impl Into<PathBuf>) -> Self {
        Self {
            app_data_dir: app_data_dir.into(),
        }
    }

    pub fn app_data_dir(&self) -> &Path {
        &self.app_data_dir
    }

    /// 기존 위치·이름 그대로(FR-002).
    pub fn projects_file(&self) -> PathBuf {
        self.app_data_dir.join("projects.json")
    }

    /// 서버 소유 변경 기록. 사용자 프로젝트 디렉터리에는 절대 쓰지 않는다.
    pub fn ledger_file(&self) -> PathBuf {
        self.app_data_dir.join("workbench").join("ledger.sqlite")
    }

    pub fn ensure_dirs(&self) -> io::Result<()> {
        fs::create_dir_all(&self.app_data_dir)?;
        if let Some(parent) = self.ledger_file().parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composes_expected_paths() {
        let paths = DataPaths::new("/tmp/aw-data");
        assert_eq!(
            paths.projects_file(),
            PathBuf::from("/tmp/aw-data/projects.json")
        );
        assert_eq!(
            paths.ledger_file(),
            PathBuf::from("/tmp/aw-data/workbench/ledger.sqlite")
        );
    }

    #[test]
    fn ensure_dirs_creates_ledger_parent() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::new(dir.path().join("data"));
        paths.ensure_dirs().unwrap();
        assert!(paths.ledger_file().parent().unwrap().is_dir());
    }
}
