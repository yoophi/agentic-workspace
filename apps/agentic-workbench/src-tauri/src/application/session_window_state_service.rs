use crate::domain::{
    session_window_state::{SessionWindowState, VisibleArea, WindowBounds},
    session_window_state_repository::SessionWindowStateRepository,
};

/// 창이 화면 밖으로 완전히 밀려나지 않도록 보장할 최소 노출 크기.
const MINIMUM_VISIBLE_PX: i32 = 120;

pub fn get_bounds(
    repository: &impl SessionWindowStateRepository,
    working_directory: &str,
) -> Result<Option<WindowBounds>, String> {
    let working_directory = normalize_directory(working_directory)?;
    Ok(repository
        .load_states()?
        .into_iter()
        .find(|state| state.working_directory == working_directory)
        .map(|state| state.bounds))
}

pub fn save_bounds(
    repository: &impl SessionWindowStateRepository,
    working_directory: &str,
    bounds: WindowBounds,
) -> Result<(), String> {
    let working_directory = normalize_directory(working_directory)?;
    if bounds.width == 0 || bounds.height == 0 {
        return Ok(());
    }

    let mut states = repository.load_states()?;
    states.retain(|state| state.working_directory != working_directory);
    states.push(SessionWindowState {
        working_directory,
        bounds,
    });
    repository.save_states(&states)
}

/// 저장된 창 상태를 현재 화면 구성에서 접근 가능한 위치·크기로 보정한다.
///
/// 모니터가 사라졌거나 해상도가 줄어든 경우에도 제목 표시줄을 잡을 수 있어야 하므로,
/// 창 크기를 화면 안으로 줄인 뒤 위치를 화면 경계로 끌어당긴다. 저장 값 자체는 바꾸지 않는다.
/// (SC-012)
pub fn fit_bounds_to_visible_areas(
    bounds: WindowBounds,
    areas: &[VisibleArea],
    minimum_width: u32,
    minimum_height: u32,
) -> Option<WindowBounds> {
    if areas.is_empty() {
        return None;
    }

    // 창 중심이 속한 화면을 우선하고, 없으면 겹치는 면적이 가장 큰 화면을 쓴다.
    let area = areas
        .iter()
        .find(|area| contains_center(area, &bounds))
        .or_else(|| areas.iter().max_by_key(|area| overlap_area(area, &bounds)))
        .copied()?;

    let width = bounds
        .width
        .min(area.width)
        .max(minimum_width.min(area.width));
    let height = bounds
        .height
        .min(area.height)
        .max(minimum_height.min(area.height));

    // 오른쪽·아래로 밀려난 창은 화면 안으로 당기고, 그래도 넘치면 화면 시작점에 맞춘다.
    let max_x = area.right() - width as i32;
    let max_y = area.bottom() - height as i32;
    let x = bounds.x.clamp(area.x.min(max_x), area.x.max(max_x));
    let y = bounds.y.clamp(area.y.min(max_y), area.y.max(max_y));

    Some(WindowBounds {
        x,
        y,
        width,
        height,
    })
}

fn contains_center(area: &VisibleArea, bounds: &WindowBounds) -> bool {
    let center_x = bounds.x.saturating_add(bounds.width as i32 / 2);
    let center_y = bounds.y.saturating_add(bounds.height as i32 / 2);
    (area.x..area.right()).contains(&center_x) && (area.y..area.bottom()).contains(&center_y)
}

fn overlap_area(area: &VisibleArea, bounds: &WindowBounds) -> i64 {
    let overlap_width = (area
        .right()
        .min(bounds.x.saturating_add(bounds.width as i32))
        - area.x.max(bounds.x))
    .max(0) as i64;
    let overlap_height = (area
        .bottom()
        .min(bounds.y.saturating_add(bounds.height as i32))
        - area.y.max(bounds.y))
    .max(0) as i64;
    // 완전히 벗어난 창도 비교 대상이 되도록 최소 노출 기준을 가중치로 더한다.
    overlap_width * overlap_height + overlap_width.min(MINIMUM_VISIBLE_PX as i64)
}

fn normalize_directory(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err("Working directory is required.".to_string())
    } else {
        Ok(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::HashMap};

    use super::*;

    #[derive(Default)]
    struct InMemoryRepository {
        states: RefCell<Vec<SessionWindowState>>,
    }

    impl SessionWindowStateRepository for InMemoryRepository {
        fn load_states(&self) -> Result<Vec<SessionWindowState>, String> {
            Ok(self.states.borrow().clone())
        }

        fn save_states(&self, states: &[SessionWindowState]) -> Result<(), String> {
            *self.states.borrow_mut() = states.to_vec();
            Ok(())
        }
    }

    fn bounds(x: i32, y: i32, width: u32, height: u32) -> WindowBounds {
        WindowBounds {
            x,
            y,
            width,
            height,
        }
    }

    fn area(x: i32, y: i32, width: u32, height: u32) -> VisibleArea {
        VisibleArea {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn saves_and_reads_bounds_per_worktree() {
        let repository = InMemoryRepository::default();

        save_bounds(&repository, "/repo/tree-a", bounds(10, 20, 1200, 900)).expect("save a");
        save_bounds(&repository, "/repo/tree-b", bounds(30, 40, 1000, 700)).expect("save b");

        assert_eq!(
            get_bounds(&repository, "/repo/tree-a").expect("read a"),
            Some(bounds(10, 20, 1200, 900))
        );
        assert_eq!(
            get_bounds(&repository, "/repo/tree-b").expect("read b"),
            Some(bounds(30, 40, 1000, 700))
        );
    }

    #[test]
    fn replaces_only_the_matching_worktree_record() {
        let repository = InMemoryRepository::default();

        save_bounds(&repository, "/repo/tree-a", bounds(10, 20, 1200, 900)).expect("save a");
        save_bounds(&repository, "/repo/tree-b", bounds(30, 40, 1000, 700)).expect("save b");
        save_bounds(&repository, "/repo/tree-a", bounds(50, 60, 1400, 1000)).expect("update a");

        assert_eq!(
            get_bounds(&repository, "/repo/tree-a").expect("read a"),
            Some(bounds(50, 60, 1400, 1000))
        );
        assert_eq!(
            get_bounds(&repository, "/repo/tree-b").expect("read b"),
            Some(bounds(30, 40, 1000, 700))
        );
        assert_eq!(repository.load_states().expect("load").len(), 2);
    }

    #[test]
    fn trims_working_directory_and_rejects_empty() {
        let repository = InMemoryRepository::default();

        save_bounds(&repository, "  /repo/tree-a  ", bounds(10, 20, 1200, 900)).expect("save");

        assert_eq!(
            get_bounds(&repository, "/repo/tree-a").expect("read"),
            Some(bounds(10, 20, 1200, 900))
        );
        assert!(save_bounds(&repository, "   ", bounds(0, 0, 100, 100)).is_err());
        assert!(get_bounds(&repository, "").is_err());
    }

    #[test]
    fn ignores_degenerate_sizes() {
        let repository = InMemoryRepository::default();

        save_bounds(&repository, "/repo/tree-a", bounds(10, 20, 0, 900)).expect("save zero width");
        save_bounds(&repository, "/repo/tree-a", bounds(10, 20, 1200, 0))
            .expect("save zero height");

        assert_eq!(get_bounds(&repository, "/repo/tree-a").expect("read"), None);
    }

    #[test]
    fn unknown_worktree_has_no_bounds() {
        let repository = InMemoryRepository::default();

        assert_eq!(
            get_bounds(&repository, "/repo/missing").expect("read"),
            None
        );
    }

    #[test]
    fn keeps_bounds_that_already_fit() {
        let fitted = fit_bounds_to_visible_areas(
            bounds(100, 100, 1200, 900),
            &[area(0, 0, 1920, 1080)],
            980,
            680,
        );

        assert_eq!(fitted, Some(bounds(100, 100, 1200, 900)));
    }

    #[test]
    fn shrinks_bounds_larger_than_the_screen() {
        let fitted = fit_bounds_to_visible_areas(
            bounds(0, 0, 3000, 2000),
            &[area(0, 0, 1440, 900)],
            980,
            680,
        );

        assert_eq!(fitted, Some(bounds(0, 0, 1440, 900)));
    }

    #[test]
    fn pulls_offscreen_bounds_back_into_view() {
        let fitted = fit_bounds_to_visible_areas(
            bounds(5000, 4000, 1200, 900),
            &[area(0, 0, 1920, 1080)],
            980,
            680,
        );

        assert_eq!(fitted, Some(bounds(720, 180, 1200, 900)));
    }

    #[test]
    fn pulls_negative_bounds_back_into_view() {
        let fitted = fit_bounds_to_visible_areas(
            bounds(-4000, -3000, 1200, 900),
            &[area(0, 0, 1920, 1080)],
            980,
            680,
        );

        assert_eq!(fitted, Some(bounds(0, 0, 1200, 900)));
    }

    #[test]
    fn keeps_window_on_the_monitor_that_holds_its_center() {
        // 두 번째 모니터(오른쪽)에 있던 창은 그 모니터에 그대로 남는다.
        let fitted = fit_bounds_to_visible_areas(
            bounds(2000, 100, 1200, 900),
            &[area(0, 0, 1920, 1080), area(1920, 0, 1920, 1080)],
            980,
            680,
        );

        assert_eq!(fitted, Some(bounds(2000, 100, 1200, 900)));
    }

    #[test]
    fn falls_back_to_the_most_overlapping_monitor_when_center_is_gone() {
        // 오른쪽 모니터가 사라진 구성. 남은 화면 안으로 보정한다.
        let fitted = fit_bounds_to_visible_areas(
            bounds(2600, 100, 1200, 900),
            &[area(0, 0, 1920, 1080)],
            980,
            680,
        );

        assert_eq!(fitted, Some(bounds(720, 100, 1200, 900)));
    }

    #[test]
    fn respects_minimum_size_even_on_a_small_screen() {
        let fitted =
            fit_bounds_to_visible_areas(bounds(0, 0, 500, 400), &[area(0, 0, 1440, 900)], 980, 680);

        assert_eq!(fitted, Some(bounds(0, 0, 980, 680)));
    }

    #[test]
    fn caps_minimum_size_to_the_available_screen() {
        // 최소 크기보다 작은 화면에서는 화면 크기까지만 늘린다.
        let fitted =
            fit_bounds_to_visible_areas(bounds(0, 0, 500, 400), &[area(0, 0, 800, 600)], 980, 680);

        assert_eq!(fitted, Some(bounds(0, 0, 800, 600)));
    }

    #[test]
    fn has_no_result_without_any_visible_area() {
        assert_eq!(
            fit_bounds_to_visible_areas(bounds(0, 0, 1200, 900), &[], 980, 680),
            None
        );
    }

    /// 실제 앱에서 발견한 회귀: 물리 픽셀 값이 저장되면(Retina 2x) 논리 단위로 해석되어
    /// 창이 화면보다 훨씬 크게 열린다. 저장 경계에서 논리 단위로 변환하는 것이 1차 방어이고,
    /// 이 보정이 2차 방어로 남아 화면 밖으로 나가는 창을 막는다.
    #[test]
    fn clamps_bounds_that_were_stored_in_physical_pixels() {
        // 3024x1898 물리 = 1512x949 논리(2x). 논리 화면은 1512x982.
        let fitted = fit_bounds_to_visible_areas(
            bounds(824, 824, 3024, 1898),
            &[area(0, 0, 1512, 982)],
            980,
            680,
        );

        assert_eq!(fitted, Some(bounds(0, 0, 1512, 982)));
    }

    #[test]
    fn corrupt_record_for_one_worktree_does_not_hide_others() {
        // 저장소가 일부 레코드를 잃은 상태에서도 남은 Worktree 설정은 그대로 읽힌다.
        let repository = InMemoryRepository::default();
        save_bounds(&repository, "/repo/tree-a", bounds(10, 20, 1200, 900)).expect("save a");
        save_bounds(&repository, "/repo/tree-b", bounds(30, 40, 1000, 700)).expect("save b");

        let mut kept: HashMap<String, WindowBounds> = HashMap::new();
        for state in repository.load_states().expect("load") {
            kept.insert(state.working_directory, state.bounds);
        }
        repository
            .save_states(&[SessionWindowState {
                working_directory: "/repo/tree-b".to_string(),
                bounds: kept["/repo/tree-b"],
            }])
            .expect("save subset");

        assert_eq!(
            get_bounds(&repository, "/repo/tree-a").expect("read a"),
            None
        );
        assert_eq!(
            get_bounds(&repository, "/repo/tree-b").expect("read b"),
            Some(bounds(30, 40, 1000, 700))
        );
    }
}
