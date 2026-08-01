use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
pub struct DataManagementService {
    base: PathBuf,
}
impl DataManagementService {
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }
    pub fn trash_path(&self, path: &Path) -> Result<PathBuf, String> {
        let trash = self.base.join("trash");
        fs::create_dir_all(&trash).map_err(|e| e.to_string())?;
        let target = trash.join(format!(
            "{}-{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            path.file_name().unwrap_or_default().to_string_lossy()
        ));
        fs::rename(path, &target).map_err(|e| e.to_string())?;
        Ok(target)
    }
    pub fn purge_expired(&self) -> Result<usize, String> {
        let trash = self.base.join("trash");
        if !trash.exists() {
            return Ok(0);
        }
        let mut count = 0;
        for entry in fs::read_dir(trash).map_err(|e| e.to_string())?.flatten() {
            if entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .unwrap_or_default()
                > Duration::from_secs(7 * 86400)
            {
                if entry.path().is_dir() {
                    fs::remove_dir_all(entry.path()).map_err(|e| e.to_string())?
                } else {
                    fs::remove_file(entry.path()).map_err(|e| e.to_string())?
                }
                count += 1
            }
        }
        Ok(count)
    }
    pub fn restore(&self, trashed: &Path, destination: &Path) -> Result<(), String> {
        if !trashed.starts_with(self.base.join("trash")) {
            return Err("restore source is outside trash".into());
        }
        if destination.exists() {
            return Err("restore destination already exists".into());
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?
        }
        fs::rename(trashed, destination).map_err(|e| e.to_string())
    }
    pub fn enforce_quota(&self, maximum_bytes: u64) -> Result<u64, String> {
        let mut files = Vec::new();
        for folder in [self.base.join("reviews/snapshots"), self.base.join("trash")] {
            collect_files(&folder, &mut files)?
        }
        files.sort_by_key(|path| path.metadata().and_then(|m| m.modified()).ok());
        let mut total = files
            .iter()
            .filter_map(|p| p.metadata().ok().map(|m| m.len()))
            .sum::<u64>();
        for path in files {
            if total <= maximum_bytes {
                break;
            }
            let size = path.metadata().map(|m| m.len()).unwrap_or(0);
            fs::remove_file(path).map_err(|e| e.to_string())?;
            total = total.saturating_sub(size)
        }
        Ok(total)
    }
}
fn collect_files(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(path).map_err(|e| e.to_string())?.flatten() {
        if entry.path().is_dir() {
            collect_files(&entry.path(), output)?
        } else {
            output.push(entry.path())
        }
    }
    Ok(())
}
#[cfg(test)]
#[path = "data_management_service_test.rs"]
mod tests;
