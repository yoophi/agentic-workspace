# Quickstart: 사용 가능한 명령 요약과 조회

## Focused Verification

```bash
pnpm --filter agentic-workbench exec vitest run \
  src/entities/agent-run/model/prompt-autocomplete.test.ts \
  src/entities/agent-run/model/format.test.ts \
  src/features/agent-run/model/run-panel-state.test.ts \
  src/features/agent-run/ui/agent-run-panel.test.tsx
```

## App Verification

```bash
pnpm --filter agentic-workbench check-types
pnpm --filter agentic-workbench test
```

Backend mapper에 typed `AvailableCommands` event를 추가한 경우에만 다음을 추가 실행한다.

```bash
cargo test -p agentic-workbench session_update_mapper
```

## Manual Verification

1. 앱을 실행한다.

   ```bash
   pnpm --filter agentic-workbench tauri:dev
   ```

2. 3개 이상의 command가 포함된 `available_commands_update`를 발생시키거나 fixture/replay로 주입한다.
   - timeline에 전체 raw JSON block이 생기지 않는다.
   - run/session header 또는 세션 정보 영역에 command count summary가 보인다.

3. command detail view를 연다.
   - command 이름이 누락 없이 보인다.
   - description이 있으면 같이 보인다.
   - input hint가 있으면 인자 필요 여부를 알 수 있게 보인다.

4. `$speckit-implement`, `/status`, `review`처럼 prefix가 다른 command를 포함한 목록을 확인한다.
   - 유효한 command 이름이 모두 표시된다.

5. 빈 목록과 malformed 목록을 확인한다.
   - visible runtime error가 없어야 한다.
   - 유효한 command만 표시하거나 empty fallback이 보인다.
   - raw payload 전체가 timeline에 표시되지 않는다.

6. 일반 raw diagnostic event를 확인한다.
   - command update가 아닌 raw event는 기존 raw timeline 동작을 유지한다.
