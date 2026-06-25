use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryChangeEvent {
    pub repository_id: String,
    pub path: String,
}

impl RepositoryChangeEvent {
    pub fn new(repository_id: String, path: String) -> Self {
        Self {
            repository_id,
            path,
        }
    }
}
