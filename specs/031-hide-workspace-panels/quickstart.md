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

자세한 상태와 경계는 [data-model.md](./data-model.md), UI 동작과 호출 계약은 [contracts/workspace-layout-ui.md](./contracts/workspace-layout-ui.md)를 따른다.
