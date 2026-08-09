use crate::{
    domain::global_preferences::GlobalPreferences,
    ports::preferences_repository::PreferencesRepository,
};
pub struct PreferencesService<R> {
    repository: R,
}
impl<R: PreferencesRepository> PreferencesService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
    pub fn load(&self) -> Result<GlobalPreferences, String> {
        self.repository.load()
    }
    pub fn save(
        &self,
        value: &GlobalPreferences,
        expected: u64,
    ) -> Result<GlobalPreferences, String> {
        value.validate()?;
        self.repository.save(value, expected)
    }
    pub fn reset(&self) -> Result<GlobalPreferences, String> {
        self.repository.reset()
    }
}
#[cfg(test)]
#[path = "preferences_service_test.rs"]
mod tests;
