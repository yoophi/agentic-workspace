# 설계 리뷰 (042)

## 리뷰 1 — OpenCodeReview delegate

- 실행: `ocr delegate preview --from 2e7f359 --to HEAD` → reviewable 1 / 8(OCR은 md를 대상에서 뺀다: `.specify/feature.json`만). `ocr delegate rule .specify/feature.json`(JSON 규칙). 설계 문서 자체는 OCR 규칙 밖이므로, 요청 범위(spec·plan·research·contracts·data-model·quickstart ↔ main `2e7f359` 코드 대조)를 직접 리뷰했다.

| # | 등급 | 내용 | 처리 |
|---|---|---|---|
| D1 | High | R4·contracts가 표 발급 때 "cursor 0개·상한 초과는 오늘 `events` 문구로 400"이라 했다. cursor 상한은 **hub 런타임 설정**(`EventHubLimits.max_cursors`, 기본 64, fixture `cursors-over-limit`는 2로 낮춤 — `event_hub/mod.rs:60,517`)이라 server 크레이트가 알 수 없다. 발급 판정과 `events` 판정이 어긋나 두 경로 parity가 깨진다 | 발급은 인증·본문 형식·**메모리 안전용 고정 상한(1,024, hub 어떤 설정보다 큼)**만 본다. 0개·hub 상한은 연결 때 `events`가 오늘 문구로 `fault` 프레임을 낸다(판정 한 곳). R4·contracts §3 수정 |
| D2 | Medium | R13 표가 `bench.open`을 "재시작 뒤 재시도 → notFound"로 묶었다. 사실은 다르다: `bench.open`은 작업대가 필요 없는 명령이라 재시작 뒤 같은 키 재시도는 **새 작업대를 연다**(세대 범위 기록이 사라짐). 영속 효과가 없어(작업대는 메모리) 재적용 해가 없지만, 증거 테스트가 "새 작업대 id, 이전 작업대 없음"을 단정해야 한다 | R13 표에 `bench.open` 행 분리, 증거 기대값 명시 |
| D3 | Medium | debug 전용 진단·probe가 release에 새지 않는다는 **검증 수단**이 없었다. Tauri `generate_handler!` 목록은 항목별 `cfg`를 받지 않아, `report_http_probe`를 등록하는 방식에 따라 release에 남을 수 있다 | debug/release에 따라 `invoke_handler`를 두 벌로 조립(`#[cfg]` 블록)하고, quickstart §2에 release 바이너리에서 `AW_HTTP_DIAGNOSTIC_FILE`·`AW_HTTP_WEBVIEW_PROBE_FILE`·`report_http_probe` 문자열 0건 확인 추가 |
| D4 | Medium | 배포 빌드 WebView의 실제 Origin이 `tauri://localhost`인지(WKWebView가 custom scheme에서 `null`을 보낼 가능성) 확인 전이다. R7은 `null`을 거절하므로, 틀리면 4단계에서 데스크톱 연결이 전부 막힌다 | 위험으로 기록: probe는 dev Origin만 자동 확인한다. 배포 Origin 실측은 4단계 전환 전 필수 게이트로 research R7·quickstart §4에 명시(추정 wildcard나 `null` 허용으로 미리 풀지 않음) |
| D5 | Low→기록 | 데스크톱 토큰 principal은 `AuthenticatedPrincipal::desktop()`과 같은 주체라 HTTP로 연 작업대와 Tauri 경로 작업대가 같은 주체다(의도: 단일 사용자, 세대 멱등 기록 공유). 진단 토큰도 같은 주체라 debug에서 파일을 읽을 수 있는 같은 OS 사용자는 데스크톱 권한을 얻는다 | 정본 위협 모델(같은 OS 사용자 malware는 범위 밖)과 일치 — R12에 명시 |

확인했고 결함이 아닌 것:
- WS 기록→실시간 경계는 `EventHub::subscribe`의 원자적 등록이 보장한다(039 `event_subscription_race.rs`). WS 어댑터는 스트림을 그대로 흘린다 — 1,000회 경합 시험은 어댑터가 프레임을 빠뜨리지 않는지를 본다.
- MCP 토큰 폐기: `CapabilityRegistry::resolve`를 요청마다 부르므로 폐기가 즉시 반영된다(041 `revoke_run`).
- 데스크톱 WebView(`http://localhost:1420`)에서 `http://127.0.0.1:<port>`로의 fetch는 교차 출처라 preflight가 필요하다 — contracts §5가 `authorization, content-type`을 허용한다. `tauri.conf.json`에 CSP가 없어 연결이 막히지 않는다(4단계에서 CSP를 넣을 때 `connect-src`에 루프백 포함 필요 — 기록).
- 세대 멱등 기록은 principal 주체 범위라 in-process와 HTTP가 공유한다(`epoch.rs` scope 계산).
- harness 교체: 기존 API(`Harness::spawn`·`call`·`subscribe`·`token_for`)를 유지하고 WS만 표 발급 → 연결로 바꾸면 fixture 기대값은 그대로다(D1 반영 조건).

## 리뷰 2 — Codex adversarial review

- 실행: 사용자가 `/codex:adversarial-review --wait …`를 직접 호출(Skill 도구의 codex 호출은 `disable-model-invocation`으로 막혀 있음). 대상: 브랜치 diff vs main(설계 문서 9파일). 판정 needs-attention.

| # | 등급 | 내용 | 처리 |
|---|---|---|---|
| C1 | High | R13은 "네트워크는 새 중단 지점을 만들지 않는다"고 가정하고 재시작 뒤 재시도만 검증했다. 그러나 `EpochIdempotency::run`은 성공 뒤에만 결과를 기록하고 취소 시 진행 중 표를 정리하며, `spawn_blocking` 저장은 호출 future가 사라져도 진행된다. HTTP 연결 단절로 handler가 취소되면 효과는 반영되고 멱등 기록은 없어 같은 키가 다시 실행된다. 재시작 시험은 작업대가 사라져 `notFound`라 이를 잡지 못한다 | **코드로 확인**(`epoch_idempotency.rs:126–144`, `OrchestrationRuntime::blocking`). research R17 신설: 어댑터(router·AW MCP 서버)가 `Workbench.call`을 **서버 소유 분리 task**에서 끝까지 실행하고 연결 future는 결과만 기다린다 — 연결이 끊겨도 실행·멱등 기록 완료, 재시도는 진행 중 슬롯에서 기다렸다가 저장 결과. R13 공개 조건에 "요청 취소 판정 증거" 추가. 공개 게이트 시험: 효과 진행 중 연결 단절 → 같은 서버·작업대 유지 → 같은 키 재시도 → 효과 1회(세대 범위 `run.sendPrompt`, 파일 영속 orchestration 변경, ledger `run.start`, MCP agent 도구), 분리 실행 제거 변이로 실패 확인. contracts §3·quickstart §1·plan 단계 3·5 갱신. 같은 결함이 오늘 MCP 서버에도 잠재함을 기록 |
