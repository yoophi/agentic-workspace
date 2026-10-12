# 공유 Workbench 데스크톱 연결 Module

box.md gh-3의 서브 task gh-21에서 AW의 `infrastructure/server_client.rs` 구현과 회귀 테스트를 `crates/workbench-desktop-client`로 이동했다. AW의 기존 파일에는 재수출과 앱별 실행 파일 탐색 Adapter만 남긴다. 두 앱이 연결 복구와 종료 절차를 같은 소스에서 소비한다.

## 소비 계약

- `DesktopConnection::new(DesktopConnectionConfig)`는 서버 데이터 디렉토리, 실행 파일, Tokio runtime handle, 종료 대기 상한을 받는다. runtime은 연결 객체와 종료 처리가 끝날 때까지 살아 있어야 한다. 라이브러리에는 Tauri 의존성이 없다.
- `window_connection(WindowIdentity)`는 서버 ensure·신원 확인·임대 확보·창 토큰 발급을 수행한다. 반환값은 camelCase `baseUrl`, `token`, `expiresAt`, `incarnation`만 포함한다. owner token과 descriptor는 공개하지 않는다. 반환값은 화면의 기존 workbench-client에 전달하며 로그나 localStorage에 쓰지 않는다.
- 창 label과 origin은 native shell에서 확인한다. incarnation은 창을 생성할 때 새 UUID로 발급한다. 서로 다른 앱과 창에 같은 incarnation을 재사용하면 같은 서버 주체를 가리키므로 금지한다. AW의 기존 window_principals 생성 규칙을 유지한다.
- `retire_window(identity, close_bench)`는 해당 창만 폐기한다. 동기 Destroyed hook은 `retire_window_detached(label, incarnation, close_bench)`를 이용한다. 호출 즉시 pending 수를 올리고 주입한 runtime에 처리를 맡기므로 현재 thread가 Tokio 안에 있지 않아도 된다.
- 정상 앱 종료에서는 `retire_windows_for_exit(windows)`를 먼저 호출한 뒤 `shutdown()`을 기다린다. 창 토큰을 폐기하되 작업대를 닫지 않고 자기 임대만 해제한다. 공유 서버 및 다른 클라이언트 작업대를 종료하지 않는다. 대기 상한은 기존 AW의 2초 기본값을 유지한다.
- `server_instance`와 `open_bench_on`은 AW의 기존 창-작업대 대응 표를 유지하는 호환 Interface다. box.md의 채팅 명령과 bench.open은 브라우저의 HTTP 클라이언트를 통해 수행한다. `pending_retirements`는 shell 종료 진단용 수치만 반환한다.
- `discover_server_executable(dev_target)`는 명시 환경 변수, 앱 실행 파일 옆, 소비자가 전달한 개발 target 디렉토리 순으로 탐색한다. crate 이동 때문에 AW의 개발 경로가 바뀌지 않도록 앱 Adapter가 기존 경로를 전달한다.

## 검증

`cargo test -p workbench-desktop-client`: 기존 22개 회귀 시험과 공개 Interface의 동기 hook·다른 클라이언트 보존 시험. 실제 HTTP host를 사용하며 토큰 폐기, 임대 해제, 다른 창 token과 작업대 유지, owner token 미노출을 검증한다. 기존 가짜 ACP 종단 시험은 응답 유실 후 교환 및 서버 정지를 검증한다. 가짜 agent 경로는 공유 crate 위치에 맞게 수정했다.

`cargo check -p agentic-workbench --all-targets`: AW가 공유 Interface로 컴파일되는 것을 확인한다. 프런트엔드 직렬화 필드는 기존 연결 command와 동일하다.

`cargo fmt --all --check` 및 `git diff --check`: 형식과 공백을 확인한다.

이 작업은 연결 Module 추출이다. 프롬프트 없는 세션 생성 및 box.md 채팅 화면은 후속 gh-22~gh-25에서 검증한다.
