use crate::domain::appearance_preferences::AppearancePreferences;

pub trait AppearancePreferencesRepository: Send + Sync {
    fn load_preferences(&self) -> Result<AppearancePreferences, String>;
    fn save_preferences(&self, preferences: &AppearancePreferences) -> Result<(), String>;
}
