use serde::Serialize;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub version: &'static str,
    pub commit: &'static str,
    pub tag: &'static str,
    pub license: &'static str,
    pub notices: &'static str,
}
pub fn build_info() -> BuildInfo {
    BuildInfo {
        version: option_env!("MA_CALVER").unwrap_or(env!("CARGO_PKG_VERSION")),
        commit: option_env!("MA_GIT_COMMIT").unwrap_or("development"),
        tag: option_env!("MA_GIT_TAG").unwrap_or("untagged"),
        license: "MIT",
        notices: "Markdown Annotator includes open-source software. See bundled notices for details.",
    }
}
