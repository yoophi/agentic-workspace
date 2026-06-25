use crate::domain::events::RunEvent;

pub trait RunEventSink: Clone + Send + Sync + 'static {
    fn emit(&self, run_id: &str, event: RunEvent);
}
