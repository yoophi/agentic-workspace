use std::path::Path;
pub trait NativeShell {
    fn reveal(&self, path: &Path) -> Result<(), String>;
    fn open_default(&self, path: &Path) -> Result<(), String>;
    fn validated_display_path(&self, root: &Path, relative_path: &str) -> Result<String, String>;
}
