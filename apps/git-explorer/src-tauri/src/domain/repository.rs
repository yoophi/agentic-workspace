use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub id: String,
    pub name: String,
    pub path: String,
}

impl Repository {
    pub fn new(id: String, name: String, path: String) -> Self {
        Self { id, name, path }
    }
}
