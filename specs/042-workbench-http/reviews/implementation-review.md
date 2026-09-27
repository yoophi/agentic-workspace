# 042 구현 리뷰

## 1. OCR delegate-review (`--from 2e7f359 --to HEAD`, 46개 검토 대상 중 운영 코드 중심)

| # | 등급 | 지적 | 조치 |
|---|---|---|---|
| O1 | Medium | 접근 기록이 클라이언트가 보낸 `requestId`·`operation`(인증 전 요청 포함)을 그대로 찍는다. 식별자는 길이만 검사하므로 개행·공백으로 기록 줄이나 필드(`status=200`)를 위조할 수 있다 | `AccessEntry::line`이 두 값을 따옴표로 감싸 escape. 단위 `client_supplied_values_cannot_forge_lines_or_fields` |
| O2 | Medium | debug probe·진단 파일: 이미 있던 파일을 덮어쓰면 `mode(0o600)`가 적용되지 않아 기존 권한이 남는다(토큰 든 진단 파일) | 쓴 뒤 `set_permissions(0o600)` |
| O3 | Medium | MCP `requestId`를 trim해 키를 만들어 도메인에서 다른 `" r1"`·`"r1"`이 같은 키로 합쳐질 수 있다 | 원래 값으로 키, 공백뿐인 값만 제외. 단위 `request_ids_are_compared_verbatim` |
| O4 | 확인 | 열린 WebSocket이 graceful shutdown을 붙잡아 종료가 멈출 수 있는가 | 붙잡지 않음 — 회귀 시험 `http_tickets.rs` `shutdown_completes_with_open_subscriptions` 추가(통과) |

확인만 하고 고치지 않은 것: `drain::accept`의 증가 뒤 재확인은 close와 경합해도 drain 뒤에 작업이 실행되지 않는다(두 번째 확인이 거절). `RunEvent::Exit`의 `block_on`은 메인 스레드를 막지만 drain은 tokio 작업자에서 돌고, 데스크톱 bridge의 `window.eval`은 전달만 하고 기다리지 않는다(루프 종료 중이면 오류를 버린다).

## 2. Codex adversarial review (branch diff against main) — verdict: needs-attention

| # | 등급 | 지적 | 조치 |
|---|---|---|---|
| C1 | High | 불완전·미인증 요청이 앱 종료를 무기한 막는다: `Bytes` 추출이 인증·수락보다 먼저이고 본문 읽기 제한 시간이 없으며, axum graceful shutdown은 그 연결을 기다린다(AW `Exit`는 메인 스레드에서 block_on) | (1) 인증을 먼저 하고 미인증은 본문을 읽지 않고 401 (2) 본문 읽기(`read_body`)는 종료 신호와 경합해 곧바로 `503`, 평상시 제한 시간 30초(`deadlineExceeded`) (3) serve를 직접 소유하는 연결 루프로 교체 — 종료 신호 뒤 연결마다 graceful, `connection_grace`(2초) 뒤 남은 연결 task를 abort·join해 소켓·router·`Workbench` 참조까지 해제. `axum::serve`는 연결 task를 detach해 future를 버려도 남는다(axum 0.7.9 `serve.rs:422`, 사용자 검토로 확인) (4) 받아들인 호출은 연결과 무관하게 추적기로 끝까지 drain |
| C2 | Medium | 서버 종료가 upgrade된 WebSocket·구독을 정리하지 않는다(on_upgrade는 별도 task). 기존 시험은 종료 뒤 읽은 결과를 버렸다 | 구독을 별도 추적기(`subscriptions`)로 세고, 종료 신호에 루프·전송이 멈추고 close(1초 제한) 뒤 스트림·소켓을 놓는다. `serve`는 구독이 0이 된 뒤 반환. 약한 시험은 지우고 hub 구독 수·runtime 참조 수·소켓 close를 단정하는 시험으로 교체 |

**수정 전 재현**(`http_shutdown_edges.rs`, 동작 없는 설정 항목만 추가한 상태): 5건 실패 — 미인증 본문·인증 본문·멈춘 헤더 종료가 3초 안에 끝나지 않음, 본문 제한 시간 없음, 종료 뒤 구독 3개 남음(`codex-before.log`).

**수정 뒤**: 7건 통과 — 위 5건 + `accepted_calls_outlive_the_connection_grace`(유예 100ms < 받아들인 호출 600ms여도 serve는 호출·멱등 기록이 끝난 뒤 반환). 멈춘 요청 시험은 serve 반환만이 아니라 클라이언트 EOF/reset과 `Arc::strong_count(runtime)` 원복까지 단정한다. 본문 취소 시험은 유예(2초)보다 짧은 1초 제한.

**보호 제거 변이**(각각 복원 후 재실행 통과):

| 변이 | 결과 |
|---|---|
| M1 본문 읽기의 종료 분기 제거 | 인증된 멈춘 본문 시험 실패(미인증은 본문을 읽지 않으므로 통과 — 인증 우선이 별도 보호) |
| M2 연결 task를 `JoinSet` 대신 detach(`tokio::spawn`, axum과 같은 함정) | 멈춘 헤더 시험 실패: "the server left the connection open" |
| M3 WebSocket 루프·전송의 종료 분기 제거 | 구독 종료 시험 실패(종료가 끝나지 않음) |
| M4 serve의 구독 drain 제거 | 구독 종료 시험 실패: runtime 참조가 남음 |

### 게이트 중 드러난 회귀와 원인 — hello가 구독 등록보다 먼저였다

수정 뒤 전체 게이트에서 이벤트 fixture `worktree-cursor-ignored [ws]`가 항목 0개로 **결정적으로** 실패했다(격리 10/10 실패, HEAD 5/5 통과). 조사 순서와 증거:

1. hello→등록 경합인가: fixture가 쓰기 직전 hub 구독 수는 이미 1(`subs=1 > baseline=0`) — fixture의 준비 신호는 충족돼 있었다.
2. 감시 시작 직후 틈(FSEvents 워밍업)인가: 진단용으로만 쓰기 전 1초를 두면 3/3 통과. 준비 확인 probe(앱 전용 임시 디렉터리 표지를 감시 callback이 받을 때까지 대기)를 실험했지만 준비가 매번 확인됐는데도 10/10 실패 — **가설 기각, probe는 최종 diff에서 제거**.
3. 단계별 마지막 성공 지점: in-memory 회차는 원시 이벤트 → debounce → hub 발행까지 성공. ws 회차는 `a.txt`의 **원시 이벤트 자체가 없음** — 쓰기가 감시 시작보다 먼저였다.
4. 원인: hub `subscribe`는 구독 수를 먼저 올리고(한도 예약, `event_hub/mod.rs:557`) 그 뒤 감시를 시작한다(`acquire_watch`, 694). 서버는 `Workbench.events` **전에** hello를 보냈다. fixture는 구독 수를 다른 task에서 폴링하므로 감시가 걸리기 전에 썼다. HEAD는 스케줄링 운으로 통과했고, 연결 task를 serve 루프가 소유하면서 interleaving이 바뀌어 드러났다. 실제 클라이언트에도 같은 결함이다 — hello를 받아도 구독 준비를 알 수 없어 재생 없는 알림 스트림의 변경을 잃는다.
5. 수정: 구독을 등록한 **뒤** hello를 보낸다(hello = 구독 준비 완료 신호, 거절이면 hello 뒤 fault — 프레임 순서 유지). contracts §4 갱신. 임의 지연·기대값 축소 없음.
6. 증거: 이벤트 suite 격리 20/20 통과. 새 회귀 시험 `http_tickets.rs` `hello_means_the_subscription_is_ready`(hello 직후 worktree 쓰기 → 알림 수신): 수정 10/10 통과, 변이(hello를 등록 전으로) 0/10 통과(10/10 실패).

**게이트(수정 뒤, 각 한 번)**: `cargo fmt --all -- --check` 0 · `cargo clippy --workspace --all-targets -- -D warnings` 0 · `cargo test --workspace --all-targets --no-fail-fast` 0(742 passed, 0 failed, 7 ignored).

## 3. OCR delegate-review 재실행 (`--from 2381d4f --to HEAD`, Codex 반영분 11개 파일)

새 High/Medium 없음. 확인한 것: 종료 신호 뒤 `calls.close()`·`subscriptions.close()`가 graceful 신호보다 먼저, 구독 추적은 표 소모보다 먼저(종료 중이면 표를 쓰지 않고 `503`), `read_body`의 `Bytes::from_request`는 `DefaultBodyLimit`를 그대로 따른다(`http_security.rs` 413 통과), 유예 뒤 abort된 연결의 handler가 기다리던 분리 호출은 추적기로 끝까지 drain(`accepted_calls_outlive_the_connection_grace`), accept 오류는 50ms 뒤 재시도. 준비 확인 probe 실험은 원인과 무관해 최종 diff에 없다.
