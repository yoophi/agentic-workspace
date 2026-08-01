use crate::ports::native_shell::NativeShell;
use std::{path::Path, process::Command};
pub struct MacOsNativeShell;
impl MacOsNativeShell {
    fn validate(root: &Path, relative: &str) -> Result<std::path::PathBuf, String> {
        if Path::new(relative).is_absolute() || relative.split('/').any(|part| part == "..") {
            return Err("unsafe path".into());
        }
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let path = root
            .join(relative)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !path.starts_with(root) || !path.is_file() {
            return Err("path is outside root or missing".into());
        }
        Ok(path)
    }
    fn command(args: &[&std::ffi::OsStr]) -> Result<(), String> {
        let status = Command::new("/usr/bin/open")
            .args(args)
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("open failed: {status}"))
        }
    }
}
impl NativeShell for MacOsNativeShell {
    fn reveal(&self, path: &Path) -> Result<(), String> {
        Self::command(&[std::ffi::OsStr::new("-R"), path.as_os_str()])
    }
    fn open_default(&self, path: &Path) -> Result<(), String> {
        Self::command(&[path.as_os_str()])
    }
    fn validated_display_path(&self, root: &Path, relative_path: &str) -> Result<String, String> {
        Ok(Self::validate(root, relative_path)?
            .to_string_lossy()
            .into_owned())
    }
}
#[cfg(test)]
#[path = "macos_native_shell_test.rs"]
mod tests;
