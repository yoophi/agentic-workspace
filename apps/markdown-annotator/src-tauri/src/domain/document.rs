use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownDocument {
    pub file_name: String,
    pub absolute_path: String,
    pub markdown_text: String,
}

pub trait MarkdownDocumentReader {
    fn read_markdown_document(&self, path: &str) -> Result<MarkdownDocument, String>;
}
