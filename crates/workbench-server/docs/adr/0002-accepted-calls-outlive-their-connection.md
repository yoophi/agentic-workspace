---
status: accepted
date: 2026-09-27
---

# 받아들인 호출은 연결보다 오래 산다

`POST /v1/calls`(와 AW MCP `tools/call`)는 받아들인 호출을 서버 소유 task(`drain::spawn_accepted`)에서 실행한다. 연결이 끊겨 handler future가 버려져도 효과와 멱등 결과 기록은 끝나고, 같은 키로 다시 온 요청은 원 호출을 기다리거나 저장된 결과를 받는다. 종료는 새 호출 수락을 먼저 닫고(`503`) 받아들인 호출이 모두 끝난 뒤에만 반환한다 — drain에는 상한이 없고 경고 간격마다 남은 수를 기록한다. 변경 operation은 operation별 중단·재시작·단절 증거가 있을 때만 네트워크에 연다(`ExposurePolicy`, `specs/042-workbench-http/reviews/exposure-evidence.md`).

## Considered Options

- handler 안에서 직접 await — 세대 멱등 command가 효과 뒤·기록 전에 취소되면 같은 키 재시도가 효과를 다시 낸다(설계 리뷰 Codex C1, 변이로 재현: 효과 2회).
- drain 상한 뒤 반환 — 상한을 넘긴 호출은 런타임 종료와 함께 끊겨 기록이 사라진다(사용자 검토).
- 분리 실행 + 상한 없는 drain(채택) — 오래 걸리는 호출이 종료를 늦출 수 있다. 경고로 드러낸다.

## Consequences

- 소유 런타임은 `serve` future가 끝난 뒤에만 내려가야 한다. AW는 `ExitRequested`에서 종료를 미루고, macOS Quit처럼 `Exit`만 오는 경우 그 자리에서 drain을 기다린다.
- ledger 경로(intent-first `spawn_blocking`)는 원래 취소되지 않아 분리 실행과 무관하게 안전하다 — 증거 표에 따로 적는다.
- 검증: `workbench-core/tests/http_disconnect_retry.rs`(단절·종료·503, 변이 기록), `drain` 단위, AW `workbench_http::tests::exit_waits_for_accepted_http_and_mcp_calls`, AW `mcp/retry_tests.rs`.
