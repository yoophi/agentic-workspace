use crate::domain::document::{MarkdownDocument, MarkdownDocumentReader};

pub struct DocumentService<R> {
    reader: R,
}

impl<R> DocumentService<R>
where
    R: MarkdownDocumentReader,
{
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn read_markdown_file(&self, path: &str) -> Result<MarkdownDocument, String> {
        self.reader.read_markdown_document(path)
    }
}
