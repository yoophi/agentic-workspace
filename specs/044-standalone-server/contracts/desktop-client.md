# 데스크톱 thin client 계약

research R3·R6·R8·R11.

## 1. 모드

| `AW_WORKBENCH_MODE` | 동작 |
|---|---|
| 없음·`external`(기본) | 서버를 찾거나 띄워(`ensure`) 붙는다. 앱 안에 런타임을 두지 않는다. compat command는 "외부 서버 모드에서는 쓸 수 없음" 오류를 돌려준다 |
| `embedded`(개발·시험) | 043 경로(앱 안 런타임 + HTTP + compat). host crate로 조립하고 같은 `owner.lock`을 잡는다. 못 잡으면 부팅 실패 |

서버 실행 파일 탐색(외부 모드) 순서:
1. `AW_WORKBENCH_SERVER_PATH`
2. 앱 실행 파일과 같은 디렉터리의 `agentic-workbench-server`
3. 개발 빌드 산출물(`target/debug`·`target/release`)

## 2. Tauri command 변화

| command | 외부 모드 동작 |
|---|---|
| `get_workbench_connection` | 앱 시작 때 한 번 `ensure` + `lease.acquire`(10초마다 갱신). 창마다 `desktop.issueWindowToken`. 출력은 043과 같다(`baseUrl, token, expiresAt, incarnation`) |
| `ensure_window_bench(open, hint)` | 창 토큰으로 `bench.open`을 부른다(label→작업대 표는 데스크톱에 남음). 출력은 043과 같다 |
| `declare_network_delivery` / `withdraw_network_delivery` | no-op(삽입 전달이 없음) |
| `apply_window_title(title)` (신규) | 창 제목과 네이티브 메뉴 동기화(데스크톱 표현) |
| compat 서버 소유 command | `"Workbench server is external; this command is unavailable."` |
| appearance·layout·창 열기·외부 URL | 그대로 |

## 3. 창·앱 수명

| 사건 | 데스크톱이 서버에 하는 일 |
|---|---|
| 창 생성 | incarnation 발급(데스크톱), 토큰은 연결 때 발급 |
| 사용자가 창을 닫음(R8 확정 신호) | `desktop.retireWindow{closeBench:true}` |
| 앱 종료가 창을 걷어 냄 | `desktop.retireWindow{closeBench:false}`(상한 2초, 실패해도 종료) |
| 앱 종료 | `lease.release`. `close_all_benches`를 부르지 않는다 |
| 앱 강제 종료 | 없음(임대 TTL로 서버가 거둠) |

## 4. 화면

- 부팅(`bootstrapTransport`)에서 외부 모드는 호환 경로 대체가 없다. 실패하면 **연결 실패 화면**(이유 + 다시 시도)을 보여 준다. 이유 예시: 실행 파일 없음, 기동 시간 초과, 버전 불일치, 권한.
- 제목 이벤트(`workspace://mcp-window-title`)를 받으면 `apply_window_title`을 부른다.
- 교환 전달 `run.sendPrompt`에 `continuation.exchangeRequestId`를 싣는다.
