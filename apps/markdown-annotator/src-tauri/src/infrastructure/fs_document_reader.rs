use std::{fs, path::PathBuf};

use crate::domain::document::{MarkdownDocument, MarkdownDocumentReader};

pub struct FsDocumentReader;

impl MarkdownDocumentReader for FsDocumentReader {
    fn read_markdown_document(&self, path: &str) -> Result<MarkdownDocument, String> {
        let path = PathBuf::from(path);
        let markdown_text = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("untitled.md")
            .to_string();

        Ok(MarkdownDocument {
            file_name,
            absolute_path: path.to_string_lossy().to_string(),
            markdown_text,
        })
    }
}
