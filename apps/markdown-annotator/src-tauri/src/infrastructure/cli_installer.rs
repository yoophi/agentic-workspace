use std::{
    fs,
    path::{Path, PathBuf},
};
const MARKER: &str = "# managed-by: markdown-annotator\n";
pub struct CliInstaller {
    path: PathBuf,
}
impl CliInstaller {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
    pub fn status(&self, target: &Path) -> bool {
        fs::read_to_string(&self.path)
            .map(|value| value == script(target))
            .unwrap_or(false)
    }
    pub fn install(&self, target: &Path) -> Result<(), String> {
        if self.path.exists() {
            let current = fs::read_to_string(&self.path).map_err(|e| e.to_string())?;
            if !current.starts_with(MARKER) {
                return Err("existing launcher is not owned by Markdown Annotator".into());
            }
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?
        }
        fs::write(&self.path, script(target)).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?
        }
        Ok(())
    }
    pub fn remove(&self) -> Result<(), String> {
        if !self.path.exists() {
            return Ok(());
        }
        let current = fs::read_to_string(&self.path).map_err(|e| e.to_string())?;
        if !current.starts_with(MARKER) {
            return Err("refusing to remove unowned launcher".into());
        }
        fs::remove_file(&self.path).map_err(|e| e.to_string())
    }
}
fn script(target: &Path) -> String {
    format!(
        "{MARKER}#!/bin/sh\nif [ \"$#\" -eq 0 ]; then set -- \"$PWD\"; fi\nnohup \"{}\" \"$@\" >/dev/null 2>&1 &\n",
        target.display()
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn never_overwrites_or_removes_unowned_launcher() {
        let base = std::env::temp_dir().join(format!("ma-cli-installer-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let path = base.join("ma");
        fs::write(&path, "user file").unwrap();
        let installer = CliInstaller::new(&path);
        assert!(installer.install(Path::new("/Applications/MA")).is_err());
        assert!(installer.remove().is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "user file");
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn launcher_passes_cwd_when_no_argument_is_given() {
        let value = script(Path::new(
            "/Applications/Markdown Annotator.app/Contents/MacOS/markdown-annotator",
        ));
        assert!(value.contains("set -- \"$PWD\""));
    }
}
