//! orchestration 저장소 전체 경계의 구현(041 research R1).
//!
//! - 경계 = 저장 단위(파일 하나 또는 메모리 인스턴스 하나)당 lock 하나. 파일은 **정규화한 경로**로 키를 만들어,
//!   같은 파일을 여는 모든 인스턴스(과도기 AW는 호출마다 새로 연다, 런타임·테스트 인스턴스 포함)가 같은 lock을
//!   쓴다. `StorageCoordinator`의 aggregate lock은 코디네이터 인스턴스마다 따로라 이 조건을 만족하지 못하고,
//!   orchestration 파일은 코디네이터가 다루지 않으므로 보호 경계는 이것 하나뿐이다(research R1 구현 메모).
//! - lock은 `'static`으로 한 번 만들어 유지한다(transaction이 guard를 소유해야 하므로). 저장 단위 수는 작다.
//! - transaction은 `MutexGuard`를 쥐어 `Send`가 아니다 — async 코드가 await 너머로 들고 있으면 컴파일되지 않는다.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, OnceLock},
};

use crate::{
    domain::agent_orchestration::{OrchestrationError, OrchestrationSession},
    ports::orchestration_repository::OrchestrationTransaction,
};

/// 경계 안에서 쓰는 원시 입출력.
pub trait SessionStorage: Send + Sync {
    fn read_all(&self) -> Result<Vec<OrchestrationSession>, OrchestrationError>;
    fn write_all(&self, sessions: &[OrchestrationSession]) -> Result<(), OrchestrationError>;
    fn boundary(&self) -> &'static Mutex<()>;
}

pub struct BoundaryTx<'a, S: SessionStorage + ?Sized> {
    storage: &'a S,
    sessions: Vec<OrchestrationSession>,
    _guard: MutexGuard<'static, ()>,
}

impl<S: SessionStorage + ?Sized> OrchestrationTransaction for BoundaryTx<'_, S> {
    fn sessions(&mut self) -> &mut Vec<OrchestrationSession> {
        &mut self.sessions
    }

    fn commit(self) -> Result<(), OrchestrationError> {
        self.storage.write_all(&self.sessions)
    }
}

pub fn begin<S: SessionStorage + ?Sized>(
    storage: &S,
) -> Result<BoundaryTx<'_, S>, OrchestrationError> {
    let guard = hold(storage.boundary());
    let sessions = storage.read_all()?;
    Ok(BoundaryTx {
        storage,
        sessions,
        _guard: guard,
    })
}

pub fn snapshot<S: SessionStorage + ?Sized>(
    storage: &S,
) -> Result<Vec<OrchestrationSession>, OrchestrationError> {
    let _guard = hold(storage.boundary());
    storage.read_all()
}

/// 파일 저장 단위의 경계. 파일이 아직 없어도 같은 키가 되도록 부모 디렉터리를 정규화하고 파일 이름을 붙인다.
pub fn boundary_for_path(path: &Path) -> &'static Mutex<()> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, &'static Mutex<()>>>> = OnceLock::new();
    let key = normalized(path);
    let mut locks = LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    locks
        .entry(key)
        .or_insert_with(|| Box::leak(Box::new(Mutex::new(()))))
}

/// 메모리 저장 단위(인스턴스 하나)의 경계.
pub fn new_boundary() -> &'static Mutex<()> {
    Box::leak(Box::new(Mutex::new(())))
}

fn normalized(path: &Path) -> PathBuf {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => {
            let parent = if parent.as_os_str().is_empty() {
                Path::new(".")
            } else {
                parent
            };
            std::fs::canonicalize(parent)
                .map(|parent| parent.join(name))
                .unwrap_or_else(|_| path.to_path_buf())
        }
        _ => path.to_path_buf(),
    }
}

fn hold(mutex: &'static Mutex<()>) -> MutexGuard<'static, ()> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_spellings_of_one_file_share_a_boundary_even_before_it_exists() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("orchestration-sessions.json");
        let dotted = dir.path().join(".").join("orchestration-sessions.json");
        assert!(!plain.exists());
        assert!(std::ptr::eq(
            boundary_for_path(&plain),
            boundary_for_path(&dotted)
        ));
        let other = dir.path().join("other.json");
        assert!(!std::ptr::eq(
            boundary_for_path(&plain),
            boundary_for_path(&other)
        ));
    }
}
