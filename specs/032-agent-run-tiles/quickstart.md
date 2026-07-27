# Quickstart Validation: Agent Run 탭·타일 워크스페이스

## Prerequisites

```sh
corepack enable
pnpm install
```

macOS에서 AW를 개발 모드로 실행한다.

```sh
pnpm run tauri:dev:workbench
```

## Scenario 1: 기존 탭 동작 보존

1. Worktree Session을 열고 Main + Extra 1 + Extra 2를 만든다.
2. 각 panel에서 다른 agent/model/초안과 timeline 위치를 준비한다.
3. 탭을 전환하고 extra idle/running close 동작을 확인한다.

Expected:

- 기존 탭 순서와 running indicator가 유지된다.
- Main은 닫을 수 없다.
- running extra는 확인 없이 취소되지 않는다.

## Scenario 2: 탭 ↔ 타일 전환

1. 세 panel 중 하나에서 agent output을 스트리밍한다.
2. toolbar에서 타일 보기를 선택한다.
3. 각 타일의 title, run status, prompt, timeline 위치를 확인한다.
4. 다른 타일을 focus한 뒤 탭 보기로 돌아간다.

Expected:

- 세 panel이 동시에 표시된다.
- run/permission/stream이 중단되지 않는다.
- 마지막 focused tile이 active tab이 된다.
- 20회 왕복 전환 후에도 panel state가 유지된다.

## Scenario 3: 인접 타일 명령과 resize

1. Main에서 `오른쪽에 새 타일 열기`를 실행한다.
2. 새 tile에서 `아래에 새 타일 열기`를 실행한다.
3. 두 separator를 pointer와 keyboard로 조절한다.
4. 아래 tile을 닫는다.

Expected:

- 첫 extra는 Main 오른쪽, 다음 extra는 선택 extra 아래에 열린다.
- 새 타일이 focus를 받는다.
- resize는 인접 subtree에만 영향을 준다.
- close 후 빈 공간 없이 sibling subtree가 확장된다.

## Scenario 4: 사용자 런 간 통신

1. Main과 Reviewer tile을 열고 Reviewer는 idle로 둔다.
2. Main tile의 peer message dialog에서 Reviewer, `draft`를 선택해 보낸다.
3. Reviewer prompt input을 확인하고 전송한다.
4. Reviewer가 running일 때 `queue` exchange를 보낸다.

Expected:

- draft는 자동 실행 없이 정확한 target textarea에 나타난다.
- queue는 current turn을 취소하지 않고 기존 queue 동작으로 들어간다.
- source/target 양쪽 status가 accepted → delivered로 일치한다.
- target 수신이 active tile/tab을 임의로 바꾸지 않는다.

## Scenario 5: MCP agent 통신

1. 두 tile에서 agent run을 시작한다.
2. source agent에게 peer 목록 조회를 요청한다.
3. 반환된 target panel/run ID로 queue message를 보내게 한다.
4. exchange status를 조회한다.

Expected:

- `list_peer_agents`는 같은 창의 다른 open endpoint만 반환한다.
- `send_message_to_agent`는 accepted request ID를 반환한다.
- target panel이 prompt를 적용한 뒤 status가 delivered가 된다.

## Scenario 6: 격리와 race

1. 같은 worktree의 두 번째 AW window와 다른 worktree window를 연다.
2. 첫 window agent에서 다른 window panel ID를 target으로 시도한다.
3. target tile을 닫는 동시에 exchange를 보낸다.
4. 동일 request ID를 같은 payload와 다른 payload로 각각 재시도한다.

Expected:

- 다른 window/worktree target은 `scope_mismatch` 또는 `unknown_target`으로 차단된다.
- close race는 cancelled/rejected terminal state가 되고 prompt를 적용하지 않는다.
- 동일 payload 재시도는 한 번만 전달된다.
- 다른 payload 재사용은 `duplicate_conflict`다.

## Automated verification

```sh
pnpm --filter @yoophi/agentic-workbench run check-types
pnpm --filter @yoophi/agentic-workbench run test
cargo test -p agentic-workbench
cargo check -p agentic-workbench
```

Storybook:

```sh
pnpm --filter @yoophi/agentic-workbench run storybook
```

필요하면 동일 package filter로 `build-storybook`을 실행해 정적 빌드도 확인한다.

## Documentation verification

- `docs/agent-run-tile-workspace.md`가 scope, non-scope, frontend/backend 경계, exchange sequence, security checks, phases, completion criteria를 포함하는지 확인한다.
- Mermaid preview에서 layout 관계와 exchange sequence가 오류 없이 렌더링되는지 확인한다.
