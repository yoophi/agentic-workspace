use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalPreferences {
    pub schema_version: u32,
    pub revision: u64,
    pub excluded_directory_names: Vec<String>,
    pub font_size: u8,
}
impl Default for GlobalPreferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            excluded_directory_names: vec![".git".into(), "node_modules".into()],
            font_size: 16,
        }
    }
}
impl GlobalPreferences {
    pub fn validate(&self) -> Result<(), String> {
        if !(12..=32).contains(&self.font_size) {
            return Err("font size must be 12..32".into());
        }
        for name in &self.excluded_directory_names {
            if name.is_empty()
                || matches!(name.as_str(), "." | "..")
                || name.contains('/')
                || name.contains('\\')
            {
                return Err(format!("invalid exact directory name: {name}"));
            }
        }
        Ok(())
    }
}
