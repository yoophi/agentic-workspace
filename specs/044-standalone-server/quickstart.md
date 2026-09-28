# Quickstart: 044 검증

앱 데이터는 설치본과 분리한다. 개발은 identifier `com.yoophi.agentic-workbench.smoke044`, 배포 출처는 `…smoke044r`로 한다. 모든 검증은 한 번 실행하고, 로그를 남기고, 원 명령의 종료 코드를 기록한다.

## 1. 서버 단독

```bash
cargo build -p agentic-workbench-server
D=$(mktemp -d)
target/debug/agentic-workbench-server ensure --data-dir "$D"   # 준비된 서버의 안내 JSON(자격 증명 제외)
stat -f '%Lp' "$D/workbench/server/server.json"                 # 600
target/debug/agentic-workbench-server serve --data-dir "$D"     # 종료 코드 3(이미 있음)
target/debug/agentic-workbench-server stop --data-dir "$D"      # 0
```

기대:
- 동시 시작 10회에서 데이터를 여는 서버는 하나다.
- `kill -9` 뒤 `ensure`가 5초 안에 복구한다.
- 임대·활성 작업이 없으면 `--idle-timeout`이 지난 뒤 멈춘다.

## 2. 비우기 분류·실제 경로 wait-stop

- `cargo test -p workbench-core drain_class`: 분류 표와 코드 대조.
- `cargo test -p workbench-host --test wait_stop`: 권한 대기, 교환 전달(043 원장 대체 없이 실제 HTTP 흐름), orchestration 자식 보고, 대기 자식 명령. 각각 wait-stop이 끝나는지, 대조 변이에서는 끝나지 않는지 본다.
- `pnpm --filter @yoophi/agentic-workbench test:integration`: 043 소비자 코드로 wait-stop 중 교환 전달(`continuation`)과 확인.

## 3. #207

- `cargo test -p workbench-core --test bench_close_idempotency`: 결정적 재현. 수정 전 실패를 기록한다.
- `acp_permission_exit` 반복 실행 기록.

## 4. 실제 앱 (R8)

1. R8-spike 로그: 경로 (a)–(h)의 이벤트 순서를 `research.md` R8에 기록한다.
2. 043 스모크(출력 + 강제 재연결, 새로고침 1회 전달)를 외부 서버 모드로 개발·배포 출처 각각 실행한다.
3. `quit` 시나리오(관측한 종료 경로마다):
   1. run 시작
   2. 그 경로로 앱 종료
   3. PID 소멸 확인
   4. 소유자 클라이언트로 `bench.list` → run 진행 중 → `run.replay`·구독으로 출력 이어짐 → `run.cancel`
4. 대조: 창 닫기에서는 그 작업대의 run이 취소된다.
5. 서버 실행 파일이 없을 때 앱이 연결 실패 화면을 보인다.

## 5. 게이트

`cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace --all-targets`, `pnpm check-types`, `pnpm test`, `pnpm build`, 두 `test:integration`.
