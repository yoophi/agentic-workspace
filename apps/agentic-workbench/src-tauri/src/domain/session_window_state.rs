use serde::{Deserialize, Serialize};

/// Worktree Session 창의 마지막 위치와 내부 크기. 패널 레이아웃과 수명·소유 책임이 다르므로
/// 별도 모델과 저장소로 분리한다. (docs/window-state-recovery-attempts.md 권장 방향)
///
/// 단위는 항상 **논리 픽셀**이다. Tauri의 `Window::outer_position`·`inner_size`는 물리 픽셀을
/// 주고 `WebviewWindowBuilder::position`·`inner_size`는 논리 단위를 받으므로, 저장 경계에서
/// scale factor로 변환해 이 모델에는 논리 단위만 들어온다. 물리 값을 그대로 담으면 Retina
/// 화면에서 창이 두 배 크기로 복원된다.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionWindowState {
    pub working_directory: String,
    #[serde(flatten)]
    pub bounds: WindowBounds,
}

/// 화면에서 창을 실제로 잡을 수 있는 영역. 모니터 구성이 달라졌을 때 보정 기준이 된다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisibleArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl VisibleArea {
    pub fn right(&self) -> i32 {
        self.x.saturating_add(self.width as i32)
    }

    pub fn bottom(&self) -> i32 {
        self.y.saturating_add(self.height as i32)
    }
}
