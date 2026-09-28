# 데스크톱 thin client 계약

research R3·R6·R8·R11.

## 1. 모드

| `AW_WORKBENCH_MODE` | 동작 |
|---|---|
| 없음·`external`(기본) | 서버를 찾거나 띄워(`ensure`) 붙는다. 앱 안에 런타임을 두지 않는다. compat command는 "외부 서버 모드에서는 쓸 수 없음" 오류를 돌려준다 |
| `embedded`(개발·시험) | 043 경로(앱 안 런타임 + HTTP + compat). host crate로 조립하고 같은 `owner.lock`을 잡는다. 못 잡으면 부팅 실패. 잡으면 안내 파일(`mode: "embedded"`)도 쓴다. 다른 클라이언트의 `ensure`가 20초를 헛기다리지 않고, 이 앱 안 서버에 붙거나 "embedded 서버가 소유 중"을 알게 한다(설계 리뷰 D4) |

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
| 사용자가 창을 닫음(그 창의 `CloseRequested`가 종료 의도보다 먼저 옴 — R8 관측: 빨간 버튼, Close Window 메뉴, 마지막 창 닫기) | `desktop.retireWindow{closeBench:true}` |
| 앱 종료가 창을 걷어 냄(`CloseRequested` 없이 `Destroyed`, 또는 종료 의도 뒤 — R8 관측: Cmd+Q·Dock·AppleScript quit은 `Exit`만) | `desktop.retireWindow{closeBench:false}`(상한 2초, 실패해도 종료) |
| 앱 종료 | `lease.release`. `close_all_benches`를 부르지 않는다 |
| 앱 강제 종료·`SIGTERM`(R8 관측: 이벤트 없음) | 없음(임대 TTL로 서버가 거둠) |

## 4. 화면

- 부팅(`bootstrapTransport`)에서 외부 모드는 호환 경로 대체가 없다. 실패하면 **연결 실패 화면**(이유 + 다시 시도)을 보여 준다. 이유 예시: 실행 파일 없음, 기동 시간 초과, 버전 불일치, 권한.
- 제목 이벤트(`workspace://mcp-window-title`)를 받으면 `apply_window_title`을 부른다.
- 교환 전달 `run.sendPrompt`에 `continuation.exchangeRequestId`를 싣는다.
- 교환 prompt는 대기열 전송(`run.sendPrompt` + continuation)으로만 보낸다(Codex r7). 대기열에서 **지우면** 서버에 전달 포기(`exchange.discardDelivery`)를 알린다 — 먼저 대기열에서 빼(자동 전송이 집어 가지 못하게) 부르고, 실패하면 항목을 제자리로 되돌린다. 교환 항목은 **steer(즉시 전송)할 수 없다**(steer는 교환 소비를 싣지 못한다 — 버튼 비활성 + 함수 거부). run 취소로 버리는 대기열과, 거절된 steer로 run을 다시 시작할 때 옮기지 않는 교환 항목(취소한 run이 대상)도 전달 포기를 알린다. run이 끝나(완료·취소·오류) 비우는 대기열은 알리지 않는다(대상 run이 없으면 서버가 활동으로 세지 않는다).
- **취소 결과를 모를 때(Codex r8)**: 네트워크 경로의 `run.cancel`이 서버에 닿지 않았거나(notApplied) 답을 받지 못했으면(unknown) "취소됨"으로 보지 않는다. run이 살아 있을 수 있으므로 패널은 대기열(교환 항목 포함)·run·응답 대기 상태를 그대로 두고 새 run을 시작하지 않는다. 실제 상태는 복구된 run 이벤트가 맞춘다.
  - 살아 있으면(그 run이 새 turn을 받음) turn 끝에 교환이 이어 가기 표지로 전달된다.
  - 취소가 적용됐으면(그 run의 취소 끝) 끝 이벤트가 패널을 정리한다. 대상 run이 없는 교환은 서버가 세지 않는다. 거절된 steer의 재시작이었으면 그때 한 번 이어 간다(교환 항목은 옮기지 않고 전달 포기). 끝 이벤트가 결과보다 먼저 왔어도 같다.
  - 서버가 답한 거절(fault)도 재시작에서는 대기열을 버리지 않는다. 성공한 취소 뒤에만 교환 항목을 버리고 전달 포기를 알린다.
  - **대기열은 호출 전 스냅샷이 아니다(Codex r9)**: 재시작·취소는 호출 전에 대기열에서 아무것도 빼지 않는다. 취소 답을 기다리는 동안 들어온 항목(이미 확인된 새 교환 포함)은 취소가 끝나지 않으면 그대로 남고, 끝나면 같은 규칙(교환은 버리고 전달 포기, 일반 prompt는 새 run으로)을 따른다. 끝 이벤트가 이미 대기열을 비웠으면 비우기 직전의 대기열로 판단한다.
  - **재시작 의도는 시도 id 하나로 한 번만 소비한다(Codex r9)**: 한 번의 "Full restart"가 시도 id를 갖고, 취소 성공 답과 결과를 몰랐던 취소의 복구된 끝 이벤트 중 먼저 온 쪽이 재시작을 한 번 한다. 새 "Full restart"·"Cancel" 조작은 앞 의도(결과를 몰라 보류된 재시작 포함)를 대체하고, 대체된 시도의 늦은 결과는 화면 상태를 바꾸지 않는다.
  - **취소 진행은 turn 응답 대기와 다른 상태다(Codex r10)**: 취소 답을 기다리는 동안에는 취소 중인 run에 대기열을 자동 전송하지 않는다. 응답 대기는 run lifecycle(`promptSent`·`promptCompleted`·steer 결과·끝)이 정하며, 취소·전송 호출이 실패해도 그 사이 lifecycle이 바꿨으면 호출 전 값으로 되돌리지 않는다 — 기다리는 동안 turn이 끝났으면 취소가 끝나지 않은 뒤 쉬는 run에 대기한 교환이 전달된다.
  - 비우는 중(wait-stop)에는 취소가 적용되는 순간 활동이 0이 되어 서버가 멈출 수 있다. 재시작의 새 run은 새 작업이라 서버가 만들지 않는다(정상).
