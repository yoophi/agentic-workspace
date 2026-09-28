---
status: accepted
date: 2026-09-28
---

# Workbench는 독립 서버 프로세스가 소유하고, 데스크톱 없는 접근은 소유자 주체로 한다

043까지 Workbench 런타임은 데스크톱 앱 프로세스 안에서 돌았다. 앱을 끄면 run·교환·orchestration이 함께 끝났고, 앱 없이 같은 run을 볼 방법이 없었다. 044는 조립을 `crates/workbench-host`로 옮기고 독립 실행 파일 `agentic-workbench-server`가 그 조립을 소유하게 한다. 데스크톱은 그 서버에 붙는 thin client가 된다.

한 데이터 디렉터리에는 서버가 정확히 하나만 쓴다. 서버는 `workbench/server/owner.lock`을 실행 내내 쥔다. 준비된 뒤에만 사용자만 읽는(0600) 안내 파일 `server.json`을 원자적으로 쓴다. 안내 파일에는 끝점과 32바이트 **소유자 자격 증명**이 들어 있다. 이 자격 증명의 주체 `local:owner`는 모든 scope와 `server:admin`을 가진다. 작업대 소유 판정도 우회하므로, 데스크톱 없이 모든 작업대의 run을 조회·구독·취소할 수 있다. 단, agent 전용 operation(orchestration agent 도구)은 우회하지 않는다. 클라이언트는 소유자 자격 증명을 보내기 전에 서버 신원부터 확인한다. 인증 없는 `identify`의 HMAC 증명을 안내 파일의 자격 증명으로 검증한다. 창 토큰은 데스크톱(소유자)이 `desktop.issueWindowToken`으로 받는다. 창 폐기(`desktop.retireWindow`)는 그 창 주체의 tombstone을 세운다.

## Considered Options

- 서버 앱 안에 조립을 두고 AW embedded 모드를 버림 — 개발·시험용 embedded 모드(8단계에 제거)가 같은 조립을 써야 해서, 조립을 공유 crate로 뒀다.
- `workbench-server`에 bin을 추가 — `workbench-server`는 protocol만 의존한다(server ADR 0001). 경계가 깨진다.
- 데스크톱 없는 접근을 기존 `desktop` 주체로 — 작업대 소유를 우회하는 주체가 드러나지 않는다. 창 토큰과 권한이 섞인다.
- PID 파일·소켓 바인드로 단일성 판단 — PID 재사용, 임의 포트 정책과 충돌한다. 파일 잠금은 비정상 종료 뒤 OS가 푼다.
- 서버가 창 incarnation을 등록 — 데스크톱이 이미 창 수명을 안다. 서버 등록은 중복이고 늦은 정리 경합만 는다. 대신 폐기 tombstone으로 늦은 발급을 막는다.

## Consequences

- 서버는 데스크톱 프로세스 그룹과 분리해 뜬다(`process_group(0)`, null stdio, 로그는 데이터 디렉터리). 앱 종료·`tauri dev` 중단이 서버를 끝내지 않는다.
- 외부 서버 모드에서는 in-process compat 경로를 쓸 수 없다(단일 writer). 부팅 실패는 연결 실패 화면으로 드러나고 대체 경로가 없다. embedded 모드도 같은 `owner.lock`을 잡는다.
- 서버 생명주기는 서빙 → 비우기 → 정지다. 비우는 동안 새 작업은 거절하고, 끝내는 제어와 이미 약속된 이어 가기(교환 전달, 배정할 쪽이 있는 대기 task)는 받는다. 활동 작업·임대가 없으면 유휴 정지한다.
- 정지를 막지 않는 두 예외가 있다(구현 중·구현 리뷰 중 정지 계약 변경). 둘 다 작업을 버리지 않고 저장한 채 보고만 한다.
  - 배정할 쪽이 없는 대기 task는 `deferredTasks`로 보고한다. 재시작 뒤 복구·인계로 다시 배정할 수 있음을 in-process runtime 재조립 시험으로 보였다.
  - 재시도를 기다리는 실패 coordinator 알림은 시도 3회까지만 활동이다. 그 뒤에는 `stalledNotifications`로 보고한다. 진행 중 시도는 횟수와 무관하게 정지를 막는다.
  - 한계: 정지·재시작 뒤 그 알림은 새 coordinator에게 **자동으로 다시 전달되지 않는다**. 인계가 이전 세대 알림을 `superseded`로 바꾸고, 결과는 `collectReports`로만 읽힌다. N+1번째 시도에서 성공했을 coordinator라도 정지가 먼저 오면 그 알림을 받지 못한다.
  - 재시작 증거는 같은 시험 프로세스 안의 runtime 재조립이다. OS 프로세스 재시작으로는 보이지 않았다.
- 소유자 자격 증명의 보호는 파일 권한(같은 OS 사용자)에 기댄다. 다른 사용자·원격 접근은 이 결정의 범위가 아니다(6단계 CLI·원격은 별도 결정).
- 프로세스 트리 가두기, 데이터 백업·이전, 설치본 포함·서명·업데이트(5단계 (d)(e)(f))는 이 결정 뒤에도 남는다.
