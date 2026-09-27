# 이벤트 fixture

`Workbench.events` 구독 계약 fixture(039). 형식과 필수 목록은 `specs/039-workbench-events/contracts/workbench-events.md` §7. `crates/workbench-core/tests/event_contract_suite.rs`가 in-memory와 테스트 WebSocket 두 경로로 실행해 결과를 서로 비교한다. 문자열 `{{epoch}}`는 실행 중인 runtime의 세대로 치환된다. `limits`는 test-hooks로 낮출 hub 한도다.
