# 044 구현 리뷰 기록

## 5단계 완료 기준 추적 (T051)

설계 문서 `docs/client-server-architecture-research.md` §5의 5단계 완료 기준이다. **044 완료는 5단계 완료가 아니다.** "후속 미완료"는 다음 증분의 범위이며, 이 PR로 완료된 것으로 세지 않는다.

| 기준 | 044에서 완료(근거) | 후속 미완료 |
|---|---|---|
| (a) 종료 뒤 지속 | 앱 종료(Cmd+Q·Dock·AppleScript·SIGTERM) 뒤 앱 PID 소멸 → 소유자 클라이언트로 같은 run 조회·live 출력·취소, 개발·배포 출처(`app-smoke.md` T045). 서버가 시작한 turn·대기열 prompt·orchestration 알림을 끝까지 실행(`wait_stop.rs`) | CLI(6단계)로 같은 흐름. 다시 연 데스크톱이 남은 작업대에 다시 붙는 화면. 교환 전달의 서버 소유(오늘은 데스크톱 UI가 라우팅·전송, 임대가 없으면 `undeliverableExchanges`로 보고만) |
| (b) 독립 composition root | `crates/workbench-host` 조립 + `apps/agentic-workbench-server`, MCP·launch decorator의 Tauri 결합 제거, 네이티브 삽입 전달 제거 | — |
| (c) 단일 writer·생명주기 | 잠금·안내 파일·identify HMAC·ensure·시작 복구·서빙→비우기→정지·임대·유휴·정지 세 방식, 프로세스 시험(동시 10회, kill -9 복구, 권한), 연결 실패 화면(T047) | — |
| (d) 프로세스 트리 가두기 | 서버 종료 때 자식 정리는 오늘 수준 유지 | 공통 감독자, 플랫폼별 트리 가두기, 강제 종료 뒤 잔여 자식 회수 |
| (e) 데이터 이전·복원·호환 | 서버가 오늘의 앱 데이터 디렉터리를 같은 형식으로 연다 | 백업·복원, 단계적 이전, 이전 실패 되돌리기 |
| (f) 배포·업데이트 | 개발·디버그 빌드에서 서버 실행 파일 탐색 규칙(경로 변수 → 앱 옆 → 개발 산출물) | 설치본 포함(externalBin)·서명·공증, 버전별 실행 캐시, 업데이트 preflight와 활성 run 보존·quiesce |

추가 후속 미완료·미검증(완료로 세지 않음):

- 관측 불가 종료 경로(로그아웃·재시동), Windows·Linux의 종료·창 이벤트 순서와 모든 스모크.
- (b2) 자동화한 Cmd+W 한 번이 두 창을 닫는 원인, 그리고 사람이 누른 Cmd+W의 동작.
- OS 프로세스 재시작 뒤 보류 task 재배정(host 재조립 수준만 검증).
- embedded 모드와 compat command 제거(8단계).

## 구현 리뷰 범위 (T053)

OCR·Codex 구현 리뷰에 다음을 명시적으로 넣는다.

1. **대기 task 정책 변경**(구현 중 변경, research R7): 대기 task는 coordinator run이 살아 있고 바쁘거나 그에게 미전달 알림이 있을 때만 활동 작업이다. 그 밖은 `deferredTasks`로 보고만 한다. 증거와 한계는 `implementation-evidence.md`("대기 task 정책 변경과 실제 K 경로")에 있다.
2. **scheduler 복구 수정**(041부터 있던 결함): 복구가 대기열에 넣은 Ready task도 자리가 비면 `acquire`가 시작한다.
3. **관찰**: MCP 도구로 받은 `draining` 거절이 `structuredContent.code = "internalError"`로 나간다.
4. **Cmd+W (b2)** 위험과 **플랫폼 미검증**(위 목록).
5. 그 밖 044 전체: 단일 writer, identify, 소유자 우회 범위, 창 토큰 tombstone, 비우기 분류와 작업 관문, 종료 판정(`window_close_intent`), #207 tombstone, 앱 스모크 증거 범위.

## 최종 게이트 (T052)

(실행 뒤 기록)

## OCR 구현 리뷰

(실행 뒤 기록)

## Codex 구현 리뷰

(실행 뒤 기록)
