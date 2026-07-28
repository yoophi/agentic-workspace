# Worktree Session 창 상태 복원 기록

## 목적

Worktree Session 창의 크기와 위치를 Worktree별로 저장·복원하는 기능이 왜 동작하지 않았는지와
어떻게 고쳤는지를 기록한다. 같은 실수를 반복하지 않도록 근본 원인과 재발 방지 테스트를 남긴다.

## 요구 동작

- 사용자가 Worktree Session 창을 이동하거나 크기를 조절하면, 해당 Worktree의 창 위치와 내부 크기를 저장한다.
- 같은 Worktree를 다시 열면 마지막 위치와 크기로 창을 생성한다.
- 패널 레이아웃 저장이 창 상태를 지우거나, 창 상태 저장이 패널 레이아웃을 지우면 안 된다.
- 화면 구성이나 모니터가 달라진 경우에도 사용자가 접근 가능한 위치와 최소 크기로 창을 표시한다.

## 근본 원인

`save_session_window_bounds`가 창 이벤트에서 Worktree 경로를 되찾으려고 웹뷰 URL의
쿼리 문자열을 파싱했다.

```rust
let Some((_, path)) = url.query_pairs().find(|(key, _)| key == "worktreePath") else { return; };
```

그런데 세션 창 URL은 HashRouter 형식이라 라우트와 쿼리가 모두 `#` **뒤**에 들어간다.

```
http://tauri.localhost/index.html#/session/proj-1?worktreePath=%2FUsers%2Fme%2Fwork%2Ftree
                                 └─ 여기부터 전부 fragment ─────────────────────────────┘
```

`Url::query_pairs()`는 `#` **앞**의 query 컴포넌트만 파싱한다. 이 URL의 query는 `None`이므로
`query_pairs()`는 항상 빈 이터레이터이고, 위 구문은 **매번 early return** 했다. 결과적으로
창 좌표가 디스크에 한 번도 기록되지 않았고, `build_window`는 언제나 저장 값이 없다고 판단해
기본 크기(1100×820)와 기본 위치로 창을 열었다.

실제 확인 값:

| 항목 | 값 |
|---|---|
| `url.query()` | `None` |
| `url.fragment()` | `Some("/session/proj-1?worktreePath=%2F...")` |
| `url.query_pairs()` | `[]` |

## 해결

1. **경로를 URL에서 유추하지 않는다.** 창을 만드는 `window_manager`가 `label → worktree_path`
   대응을 직접 기억하고(`remember_session_worktree_path`), 창 이벤트에서 그 값을 조회한다.
   창이 사라질 때 `forget_session_window`으로 정리한다. URL 형식 변화에 더 이상 의존하지 않는다.
2. **창 상태를 전용 모델·저장소로 분리했다.** `domain/session_window_state.rs`,
   `application/session_window_state_service.rs`,
   `infrastructure/json_session_window_state_repository.rs`(`session-window-states.json`).
   `WorkspaceLayoutSettings`에서 `windowX/Y/Width/Height`를 제거했다.
   - 분리 전에는 프런트엔드가 패널 폭을 저장할 때 마운트 시점의 창 좌표를 함께 되돌려 보냈고,
     `save_layout`의 `or(existing)` 병합 때문에 그 낡은 값이 방금 저장된 좌표를 덮어썼다.
     두 값의 소유자를 나누어 이 경합을 구조적으로 없앴다.
3. **모니터 구성 변화를 보정한다.** `fit_bounds_to_visible_areas`가 창 중심이 속한 모니터를
   우선 선택하고, 없으면 겹치는 면적이 가장 큰 모니터로 크기·위치를 끌어당긴다. 저장 값 자체는
   바꾸지 않는다. 모니터보다 큰 창은 줄이고, 화면 밖으로 나간 창은 화면 안으로 당긴다. (SC-012)
4. **디스크 쓰기를 제한한다.** `Moved`/`Resized`는 드래그 중 연속으로 들어오므로
   `BOUNDS_SAVE_INTERVAL`(700ms) 간격으로만 저장하고, `CloseRequested`/`Destroyed`에서는
   간격과 무관하게 마지막 값을 기록한다. 최소화 상태와 0 크기는 저장하지 않는다.

## 실제 앱에서 추가로 발견한 결함: 물리 픽셀 / 논리 단위 불일치

위 수정으로 저장은 되기 시작했지만, 실제 앱을 띄워 확인했을 때 저장된 값이 틀렸다.

```json
{ "x": 824, "y": 824, "width": 3024, "height": 1898 }   // Retina(2x) 화면의 물리 픽셀
```

- `Window::outer_position()`·`inner_size()`는 **물리 픽셀**을 준다.
- `WebviewWindowBuilder::position()`·`inner_size()`는 **논리 단위**를 받는다.

그대로 저장하면 다음 실행 때 창을 두 배 크기·위치로 열려고 한다(2x 화면에서 6048×3796 요청).
저장 경계에서 `scale_factor()`로 논리 단위로 변환해 해결했고, 모니터 정보도 각 모니터의
scale factor로 논리 좌표로 바꿔 최소 크기 상수와 같은 단위로 비교한다.

수정 후 실제 저장 값:

```json
{ "x": 135, "y": 102, "width": 1085, "height": 810 }    // 논리 단위 (화면 1512x982 이내)
```

이 결함은 단위가 없는 숫자만 다루는 단위 테스트로는 드러나지 않는다. 실제 앱에서 창을
움직여 저장 파일을 확인하는 절차가 반드시 필요하다.

## 재발 방지

- `window_manager::tests::worktree_path_is_not_readable_from_the_url_query`
  세션 URL의 query가 비어 있고 `worktreePath`가 fragment에 있다는 사실을 고정한다.
  다시 `query_pairs()` 방식으로 되돌리면 이 테스트가 근거를 남긴다.
- `window_manager::tests::window_manager_remembers_worktree_path_per_session_label`,
  `separate_session_windows_keep_separate_worktree_paths`
  label별 경로 보관과 정리를 검증한다.
- `session_window_state_service::tests` 14건
  Worktree별 저장·격리·경로 정규화, 모니터 보정(축소·화면 안으로 당김·다중 모니터·최소 크기)을 검증한다.
- `session_window_state_service::tests::clamps_bounds_that_were_stored_in_physical_pixels`
  물리 픽셀 값이 흘러들어도 화면 밖으로 나가지 않도록 2차 방어를 고정한다. 1차 방어는
  저장 경계의 논리 단위 변환이다.

## 남은 수동 검증

자동 테스트로는 실제 창 이벤트와 OS 창 배치를 대체할 수 없다. 다음은 개발용 AW 앱에서 확인해야 한다.

1. 세션 창을 이동·리사이즈하고 닫은 뒤 같은 Worktree를 다시 열어 위치·크기가 복원되는지.
2. 서로 다른 두 Worktree가 각각의 창 상태를 독립적으로 유지하는지.
3. macOS 탭 모드(`open_as_tab`)로 만든 창도 같은 경로를 지나 저장·복원되는지.
4. 외부 모니터를 분리한 뒤 열었을 때 창이 화면 안 접근 가능한 위치로 보정되는지.
5. 창을 빠르게 드래그하는 동안 UI가 끊기지 않는지(저장 간격 확인).

## 이전 스키마

기존 `worktree-workspace-layouts.json`에 남아 있는 `windowX/windowY/windowWidth/windowHeight`
필드는 무시된다(serde가 알 수 없는 필드를 버린다). 창 상태는 새 저장소에서 다시 수집되므로
이전 기록은 사용되지 않지만, 애초에 저장된 적이 없었으므로 실사용 영향은 없다.

## 관련 파일

- `apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs`
- `apps/agentic-workbench/src-tauri/src/domain/session_window_state.rs`
- `apps/agentic-workbench/src-tauri/src/domain/session_window_state_repository.rs`
- `apps/agentic-workbench/src-tauri/src/application/session_window_state_service.rs`
- `apps/agentic-workbench/src-tauri/src/infrastructure/json_session_window_state_repository.rs`
- `apps/agentic-workbench/src-tauri/src/lib.rs`
- `specs/031-hide-workspace-panels/spec.md`
