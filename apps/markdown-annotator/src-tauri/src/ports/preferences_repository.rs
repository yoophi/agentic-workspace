use crate::domain::global_preferences::GlobalPreferences;
pub trait PreferencesRepository {
    fn load(&self) -> Result<GlobalPreferences, String>;
    fn save(
        &self,
        value: &GlobalPreferences,
        expected_revision: u64,
    ) -> Result<GlobalPreferences, String>;
    fn reset(&self) -> Result<GlobalPreferences, String>;
}
