# Quickstart: 세션 정보 메타데이터 표시

## Focused Verification

```bash
pnpm --filter agentic-workbench exec vitest run \
  src/entities/agent-run/model/format.test.ts \
  src/entities/project/lib/worktree-window-title.test.ts \
  src/features/agent-run/model/run-panel-state.test.ts \
  src/features/agent-run/ui/agent-run-panel.test.tsx \
  src/app/App.test.tsx
```

## App Verification

```bash
pnpm --filter agentic-workbench check-types
pnpm --filter agentic-workbench test
```

Backend mapper를 수정한 경우에만 다음을 추가 실행한다.

```bash
cargo test -p agentic-workbench session_update_mapper
```

## Manual Verification

1. 앱을 실행한다.

   ```bash
   pnpm --filter agentic-workbench tauri:dev
   ```

2. `title`이 있는 `session_info_update`를 발생시키거나 fixture/replay로 주입한다.
   - AW window title이 session title로 바뀐다.
   - timeline에 raw JSON block이 생기지 않는다.

3. `updatedAt`만 있는 update를 발생시킨다.
   - window title은 유지된다.
   - run/session header의 보조 metadata에 최신성 표시가 보인다.
   - active/idle status indicator는 사라지지 않는다.

4. `title`, `updatedAt`, `threadStatus`가 함께 있는 update를 발생시킨다.
   - window title, freshness label, status indicator가 각각 갱신된다.
   - timeline에는 raw payload가 표시되지 않는다.

5. 빈 title, control character title, invalid `updatedAt`을 확인한다.
   - visible runtime error가 없어야 한다.
   - 기존 title/status 표시가 잘못된 값으로 덮이지 않아야 한다.

6. project/worktree route를 바꾼다.
   - live session title이 reset되고 기본 worktree window title이 표시된다.
