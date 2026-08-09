use std::path::Path;

use crate::{
    domain::file_browser::{FileBrowserError, RootIdentity, RootScanResult},
    ports::file_browser::{DocumentReadResult, FileBrowser},
};

pub struct FileBrowserService<B> {
    browser: B,
}

impl<B: FileBrowser> FileBrowserService<B> {
    pub fn new(browser: B) -> Self {
        Self { browser }
    }

    pub fn open_root(&self, path: &Path) -> Result<RootIdentity, FileBrowserError> {
        self.browser.canonical_root(path)
    }

    pub fn read_document(
        &self,
        root_path: &Path,
        relative_path: &str,
    ) -> Result<DocumentReadResult, FileBrowserError> {
        let root = self.open_root(root_path)?;
        self.browser.read_document(&root, relative_path)
    }

    pub fn scan_root(
        &self,
        root_path: &Path,
        excluded_directory_names: &[String],
    ) -> Result<RootScanResult, FileBrowserError> {
        let root = self.open_root(root_path)?;
        self.browser.scan_root(&root, excluded_directory_names)
    }
}
