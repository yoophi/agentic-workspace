# UI 계약: Worktree Workspace 레이아웃

## 1. 패널 선택 제어

- 제어 영역은 Worktree Session 화면의 가장 오른쪽에 세로로 배치한다.
- Git, Files, Markdown, Speckit 버튼은 각각 접근 가능한 이름을 가진다.
- 각 버튼의 보이는 식별 표시는 90도 회전된 방향으로 배치한다.
- 선택되지 않은 버튼을 누르면 그 패널만 표시한다.
- 선택된 버튼을 다시 누르면 선택이 해제되며 Workspace B와 바깥 분할 핸들은 렌더링하지 않는다.
- 선택 없음 상태에서 네 버튼은 계속 표시되며, 에이전트 작업 영역은 가능한 전체 폭을 사용한다.

## 2. 크기 계약

모든 조절 가능한 가로 분할은 다음 계약을 따른다.

| 영역 | A | B | 저장 키 |
|---|---|---|---|
| 바깥 분할 | 에이전트 작업 영역 | 선택된 Workspace 영역 | `outerPanelWidthPx` |
| Git 내부 | 탐색/목록 영역 | 상세/미리보기 영역 | `panelWidthsPx.git` |
| Files 내부 | 파일 트리 영역 | 파일 미리보기 영역 | `panelWidthsPx.files` |
| Markdown 내부 | Markdown 파일 목록 | Markdown 미리보기 영역 | `panelWidthsPx.markdown` |
| Speckit 내부 | Speckit 문서 목록 | 문서 미리보기 영역 | `panelWidthsPx.speckit` |

- 관계는 항상 `A:B = *:1`이다. B의 선호 폭만 저장하고, A는 남은 공간을 사용한다.
- 저장된 B 폭은 같은 Worktree에서만 다시 적용한다.
- 다른 패널 종류의 내부 B 폭은 서로 덮어쓰지 않는다.
- 현재 컨테이너가 저장된 B 폭을 수용할 수 없으면, 표시 폭만 최소 A/B 제한 안으로 조정한다.
- B가 숨겨졌거나 내부 분할이 존재하지 않으면 해당 분할을 렌더링하거나 저장 요청을 만들지 않는다.

## 3. Tauri 호출 경계

### `get_worktree_workspace_layout`

입력: `workingDirectory: string`

출력: `WorkspaceLayoutSettings | null`

오류: 빈 Worktree 경로 또는 읽기 실패는 사용자에게 이해 가능한 오류로 전달한다.

### `save_worktree_workspace_layout`

입력: 전체 `WorkspaceLayoutSettings`

출력: 정규화되어 저장된 `WorkspaceLayoutSettings`

보장: 동일 Worktree 레코드만 교체하며 다른 Worktree의 레이아웃은 변경하지 않는다.
