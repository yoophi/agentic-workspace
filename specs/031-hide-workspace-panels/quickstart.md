# 빠른 검증 가이드: Workspace 패널 숨김 및 크기 복원

## 준비

```sh
pnpm --dir apps/agentic-workbench check-types
pnpm --dir apps/agentic-workbench test
cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml
```

개발 화면을 확인해야 하면 다음을 실행한다.

```sh
pnpm run tauri:dev:workbench
```

## 수동 검증

1. Worktree Session을 열고 오른쪽 가장자리에 Git, Files, Markdown, Speckit 세로 제어 버튼이 있는지 확인한다. 각 식별 표시는 90도 회전되어야 한다.
2. Git 버튼을 선택하고, 다시 선택한다. Workspace 콘텐츠와 바깥 분할 핸들이 사라지고 에이전트 영역이 남은 폭을 모두 사용해야 한다.
3. 선택 없음 상태에서 Files, Markdown, Speckit을 각각 선택해 해당 패널만 표시되는지 확인한다.
4. 바깥 B 폭을 조절하고 Git, Files, Markdown, Speckit 내부의 B 폭도 각각 조절한다. 세션을 닫았다가 같은 Worktree를 다시 열어 각 B 폭이 복원되는지 확인한다.
5. 다른 Worktree에서 다른 B 폭을 설정한 뒤 두 Worktree를 다시 열어 서로의 저장 폭이 섞이지 않는지 확인한다.
6. 창 폭을 줄여 저장된 폭을 그대로 적용할 수 없는 상태를 만든다. A와 B가 모두 사용 가능하게 표시되고, 창을 다시 넓혔을 때 원래 선호 B 폭이 유지되는지 확인한다.
7. 키보드로 네 제어 버튼에 이동해 선택과 선택 해제를 수행하고, 보조 기술이 버튼 이름과 선택 상태를 구분할 수 있는지 확인한다.
8. 창을 이동·리사이즈한 뒤 닫고 같은 Worktree를 다시 열어 창 위치와 크기가 복원되는지 확인한다. 외부 모니터를 분리한 상태에서도 창이 화면 안 접근 가능한 위치로 열리는지 확인한다.

자세한 상태와 경계는 [data-model.md](./data-model.md), UI 동작과 호출 계약은 [contracts/workspace-layout-ui.md](./contracts/workspace-layout-ui.md)를 따른다.

## 자동 검증 결과

`pnpm --dir apps/agentic-workbench check-types`, `pnpm --dir apps/agentic-workbench test`,
`cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml`,
`cargo fmt --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml --check`,
`git diff --check`를 실행해 모두 통과했다.

| 대상 | 결과 |
|---|---|
| `check-types` | 통과 |
| 프런트엔드 테스트 | 통과 |
| Rust 테스트 | 통과 |
| `cargo fmt --check` | 통과 |

## 앱 실행 검증 결과 (2026-07-29)

개발용 AW 앱(`VITE_DEV_SERVER_PORT=1430 pnpm tauri dev`)을 띄워 화면을 직접 조작해 확인했다.

| 시나리오 | 결과 | 확인 방법 |
|---|---|---|
| 1. 오른쪽 세로 4버튼, 90도 회전 | 통과 | 세션 창 오른쪽 가장자리에 Git·Files·Markdown·Speckit 표시 |
| 2. 재선택으로 완전 숨김 | 통과 | Git 재클릭 시 Workspace·분할 핸들이 사라지고 에이전트가 전체 폭 사용, 버튼은 유지 |
| 3. 다른 패널 선택 | 통과 | Files 선택 시 File tree/File preview만 표시 |
| 4. B 폭 저장·복원 | 통과 | 바깥 드래그 → `outerPanelWidthPx` 537→580 저장, 재적용 확인. `panelWidthsPx.files=348`이 File preview에 복원 |
| 6. 좁은 화면에서 선호 폭 보존 | 통과 | 두 내부 pane이 최소 폭 아래로 눌린 상태에서도 저장된 `files: 348`이 바뀌지 않음 |
| 8. 창 위치·크기 저장 | 통과(단위 결함 수정 후) | `session-window-states.json`에 논리 단위로 기록: `x:135 y:102 1085x810` |

검증 중 **물리 픽셀/논리 단위 불일치 결함**을 발견해 수정했다. 상세는
[창 상태 복원 기록](../../docs/window-state-recovery-attempts.md)에 있다.

## 남은 수동 검증

- 5번 Worktree 간 격리: 두 번째 Worktree 세션을 열어 서로의 저장 폭이 섞이지 않는지 확인.
- 7번 키보드·보조 기술 조작: Tab 이동과 선택/해제, 스크린리더의 이름·상태 안내 확인.
- 8번의 모니터 구성 변경(외부 모니터 분리 후 보정)과 macOS 탭 모드(`open_as_tab`) 경로.

세 항목은 창을 여러 개 띄우거나 하드웨어 구성을 바꿔야 하므로 사람이 직접 확인해야 한다.
