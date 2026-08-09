use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentIdentity {
    pub root_id: String,
    pub relative_path: String,
    pub fingerprint: String,
    pub byte_length: u64,
    pub modified_at_ms: Option<u64>,
}

impl DocumentIdentity {
    pub fn key(&self) -> String {
        format!("{}:{}", self.root_id, self.relative_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_key_is_scoped_to_root_and_path() {
        let identity = DocumentIdentity {
            root_id: "root-1".into(),
            relative_path: "docs/readme.md".into(),
            fingerprint: "f".repeat(64),
            byte_length: 10,
            modified_at_ms: None,
        };
        assert_eq!(identity.key(), "root-1:docs/readme.md");
    }
}
