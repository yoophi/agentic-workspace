use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleChangeRequest {
    pub run_id: String,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleChangeResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<TitleChangeFailureCode>,
}

impl TitleChangeResult {
    pub fn success(applied_title: String) -> Self {
        Self {
            ok: true,
            applied_title: Some(applied_title),
            reason: None,
            code: None,
        }
    }

    pub fn failure(code: TitleChangeFailureCode, reason: impl Into<String>) -> Self {
        Self {
            ok: false,
            applied_title: None,
            reason: Some(reason.into()),
            code: Some(code),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TitleChangeFailureCode {
    Unauthorized,
    UnknownRun,
    InactiveRun,
    InvalidTitle,
    WindowUnavailable,
    UnsupportedTool,
    InternalError,
}
