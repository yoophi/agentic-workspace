use crate::domain::session_window_state::SessionWindowState;

pub trait SessionWindowStateRepository {
    fn load_states(&self) -> Result<Vec<SessionWindowState>, String>;
    fn save_states(&self, states: &[SessionWindowState]) -> Result<(), String>;
}
