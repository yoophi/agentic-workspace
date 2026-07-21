# Quickstart / Validation Guide: Hushline Agent Run

이 기능이 end-to-end로 동작함을 증명하는 실행·검증 시나리오. 구현 세부는 tasks.md/구현 단계에서
다룬다. 여기서는 무엇을 어떻게 검증하는지만 기술한다.

## 전제 조건

- pnpm 9.x + Corepack, Rust 툴체인, Tauri 데스크톱 사전요건.
- 자막 생성용 외부 도구(다운로더/추출/인식 엔진)와 agent 실행 도구가 설치·구성됨.
- 브랜치 `030-hushline-monorepo-integration` 체크아웃 상태.

## Phase별 검증

### V0. 코어 추출 후 회귀 검증 (workbench)
```
pnpm --filter @yoophi/agentic-workbench check-types
cargo test -p acp-agent-core
cargo test -p agentic-workbench
```
- 기대: 공유 crate 테스트 그린 + workbench 기존 테스트 그린(무동작 리팩터).

### V1. 편입 후 hushline 무회귀 (P1 / SC-001)
```
pnpm install
pnpm build
pnpm check-types
cargo test -p hushline
pnpm --filter @yoophi/hushline tauri:dev   # 수동: 자막 생성/queue 동작 확인
```
- 기대: hushline이 공용 파이프라인에 포함되어 통과. YouTube URL 자막 생성·queue 추가/삭제·
  진행 표시가 편입 전과 동일(회귀 0건).

### V2. Walking skeleton — 스트리밍 검증 (P2 최소)
- 절차: 자막 결과 카드에서 "정리하기" → 정리 방식 입력 → 실행.
- 기대: `agent-run-event` 스트림이 도착해 진행 상태와 생성 텍스트가 실시간 표시(SC-003).
  command 1개(`start_agent_run`) + 버튼 1개로 성립.

### V3. 정리 문서 저장·재열람 (P2 / SC-002)
- 절차: 정리 run 완료 후 저장 → 목록에서 재열람.
- 기대: 원본 자막과 연결된 OrganizedDocument가 영구 저장되고 다시 열림. 저장까지 3분 이내 수행 가능.

### V4. 취소·오류 처리 (FR-006·FR-007 / SC-005·SC-006)
- 절차: (a) 진행 중 run 취소, (b) agent 도구 미설치 등 오류 유발.
- 기대: (a) run 중단 + 부분 결과 처리 안내 + registry 잔여 run 0, (b) 원인 메시지 표시 + 앱 안정.

### V5. 지식 기반 대화 (P3 / SC-004)
- 절차: 저장된 문서에서 대화 시작 → 후속 질문 2회 이상.
- 기대: 첫 답변이 문서 근거로 스트리밍, 후속 질문이 이전 맥락 반영(검증 시나리오 100%).
  창/앱 종료 시 진행 run 안전 정리(누수 0).

## 계약 테스트 (contracts/agent-run-commands.md)
```
cargo test -p acp-agent-core            # CT-1(직렬화 라운드트립), CT-4/CT-5(registry)
cargo test -p hushline                  # CT-2(cwd 검증), CT-3(소유 검증)
pnpm --filter @yoophi/agent-client test # TS 계약 스냅샷(있을 경우)
```

## 완료 기준(요약)
- V0~V1 그린: 편입·추출 회귀 없음.
- V2~V3 통과: 정리→스트리밍→저장 end-to-end.
- V4 통과: 취소/오류 안전성.
- V5 통과: 세션 내 다회 대화.
