use crate::domain::{
    document_identity::DocumentIdentity,
    file_browser::{FileBrowserError, RootIdentity, RootScanResult},
};

#[derive(Debug)]
pub struct DocumentReadResult {
    pub identity: DocumentIdentity,
    pub markdown_text: String,
}

pub trait FileBrowser: Send + Sync {
    fn canonical_root(&self, path: &std::path::Path) -> Result<RootIdentity, FileBrowserError>;
    fn scan_root(
        &self,
        root: &RootIdentity,
        excluded_directory_names: &[String],
    ) -> Result<RootScanResult, FileBrowserError>;
    fn read_document(
        &self,
        root: &RootIdentity,
        relative_path: &str,
    ) -> Result<DocumentReadResult, FileBrowserError>;
}
