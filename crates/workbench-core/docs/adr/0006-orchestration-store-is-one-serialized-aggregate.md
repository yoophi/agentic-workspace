---
status: accepted
date: 2026-09-27
---

# orchestration 저장소는 하나의 직렬화된 aggregate이며, 작업 영역 범위 lock은 두지 않는다

`orchestration-sessions.json`은 모든 작업 영역을 한 파일에 담는다. 작업 영역별로만 직렬화하면 서로 다른 작업 영역의 동시 변경이 같은 파일을 읽고-고치고-쓰면서 서로를 덮어쓴다(lost update). 그래서 모든 변경은 **파일 전체를 하나의 read-modify-write 경계** 안에서 한다: 정규화한 경로마다 프로세스 전역 lock 하나, transaction은 `begin → sessions() 수정 → commit`이고 guard가 `!Send`라 await를 넘지 못한다. `StorageCoordinator` aggregate lock은 쓰지 않는다 — 인스턴스별 lock이라 `'static` guard를 줄 수 없고, 이 파일은 `StorageCoordinator`가 다루지 않으므로 보호 경계가 둘로 갈리지 않는다(포트에는 계약만, 구현은 `infrastructure/orchestration/store_boundary.rs`).

작업 영역 범위 async lock(operation 동안 쥐는 lock)은 **두지 않는다**. 엔진 호출·`waitChildTasks` 대기·coordinator 알림 전달·작업대 닫기는 저장 경계 밖에서 await하고, 결과는 상태 조건(과제 시도·run id·revision)을 다시 확인하는 짧은 transaction으로 반영한다. 그래서 보고·취소·종료가 긴 대기에 막히지 않는다. 작업 영역 id를 모른 채 "찾기/만들기 + 이미 묶임 검사 + 묶임 표 갱신"을 해야 하는 묶기(bootstrap·recover·release)만 binding mutex를 저장 경계보다 먼저 잡는다(순서 고정: binding mutex → 저장 경계).

## Considered Options

- 작업 영역별 파일 분할 — 경계는 자연스럽지만 옛 파일 이전과 복구 가능 목록 조회가 여러 파일 스캔이 된다.
- SQLite로 이전 — 트랜잭션은 얻지만 이번 단계의 목표(서버 이관)와 무관한 저장 형식 변경이다.
- 작업 영역 범위 async lock — 경합은 줄지만 `waitChildTasks`·알림 전달·닫기 hook과 얽혀 교착·지연을 만든다(설계 리뷰 H2–H4).

## Consequences

- 서로 다른 작업 영역의 변경도 파일 쓰기 단위로 직렬화된다(파일 크기에 비례). 규모가 커지면 파일 분할이나 SQLite로 대체한다.
- 검증: `tests/orchestration_concurrency.rs`(작업 영역 여럿 × thread 8 × 100회 이상 손실 0, lock을 끈 회귀 경로는 실패), liveness 테스트 ①–⑤.
