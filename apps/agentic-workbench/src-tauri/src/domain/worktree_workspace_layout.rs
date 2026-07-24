use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLayoutSettings {
    pub working_directory: String,
    #[serde(default)]
    pub outer_panel_width_px: Option<u32>,
    #[serde(default)]
    pub panel_widths_px: BTreeMap<String, u32>,
    #[serde(default)]
    pub window_x: Option<i32>,
    #[serde(default)]
    pub window_y: Option<i32>,
    #[serde(default)]
    pub window_width: Option<u32>,
    #[serde(default)]
    pub window_height: Option<u32>,
}
