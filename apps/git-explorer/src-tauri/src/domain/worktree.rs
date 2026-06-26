use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktree {
    pub path: String,
    pub branch: Option<String>,
    pub commit: String,
    pub is_bare: bool,
    pub is_main: bool,
}

impl GitWorktree {
    pub fn new(
        path: String,
        branch: Option<String>,
        commit: String,
        is_bare: bool,
        is_main: bool,
    ) -> Self {
        Self {
            path,
            branch,
            commit,
            is_bare,
            is_main,
        }
    }
}
