//! aggregate 단위 직렬화 port. 읽기·쓰기·백업 복구가 모두 같은 lock을 거친다(research R14).

pub trait AggregateLock: Send + Sync {
    /// `aggregate`의 lock을 잡은 채 `f`를 실행한다. 같은 aggregate의 다른 호출은 끝날 때까지 기다린다.
    fn run_locked<R>(&self, aggregate: &str, f: impl FnOnce() -> R) -> R
    where
        Self: Sized;
}
