use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranch {
    pub name: String,
    pub full_name: String,
    pub is_remote: bool,
    pub is_current: bool,
    pub worktree_path: Option<String>,
}

impl GitBranch {
    pub fn new(
        name: String,
        full_name: String,
        is_remote: bool,
        is_current: bool,
        worktree_path: Option<String>,
    ) -> Self {
        Self {
            name,
            full_name,
            is_remote,
            is_current,
            worktree_path,
        }
    }
}
