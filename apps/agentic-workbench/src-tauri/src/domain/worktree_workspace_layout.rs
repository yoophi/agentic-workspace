use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Worktree Session의 분할 폭 설정. B(작은 영역)의 선호 폭만 저장하고 A의 폭은 저장하지 않는다.
///
/// 창 위치·크기는 `session_window_state`가 소유하므로 여기에 두지 않는다. 두 값을 한 레코드에
/// 담으면 프런트엔드의 폭 저장과 창 이벤트의 위치 저장이 서로의 값을 덮어쓴다.
/// (docs/window-state-recovery-attempts.md)
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLayoutSettings {
    pub working_directory: String,
    #[serde(default)]
    pub outer_panel_width_px: Option<u32>,
    #[serde(default)]
    pub panel_widths_px: BTreeMap<String, u32>,
}
