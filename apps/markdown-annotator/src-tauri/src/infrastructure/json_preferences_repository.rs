use crate::{
    domain::global_preferences::GlobalPreferences,
    ports::preferences_repository::PreferencesRepository,
};
use std::{fs, io::Write, path::PathBuf};
pub struct JsonPreferencesRepository {
    base: PathBuf,
}
impl JsonPreferencesRepository {
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }
    fn path(&self) -> PathBuf {
        self.base.join("preferences.json")
    }
}
impl PreferencesRepository for JsonPreferencesRepository {
    fn load(&self) -> Result<GlobalPreferences, String> {
        let path = self.path();
        if !path.exists() {
            return Ok(GlobalPreferences::default());
        }
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }
    fn save(
        &self,
        value: &GlobalPreferences,
        expected_revision: u64,
    ) -> Result<GlobalPreferences, String> {
        let actual = self.load()?.revision;
        if actual != expected_revision {
            return Err(format!(
                "revision conflict: expected {expected_revision}, actual {actual}"
            ));
        }
        let mut saved = value.clone();
        saved.validate()?;
        saved.revision = actual + 1;
        fs::create_dir_all(&self.base).map_err(|e| e.to_string())?;
        let temp = self
            .base
            .join(format!(".preferences-{}.tmp", std::process::id()));
        let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(&mut file, &saved).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(temp, self.path()).map_err(|e| e.to_string())?;
        Ok(saved)
    }
    fn reset(&self) -> Result<GlobalPreferences, String> {
        let current = self.load()?;
        self.save(&GlobalPreferences::default(), current.revision)
    }
}
