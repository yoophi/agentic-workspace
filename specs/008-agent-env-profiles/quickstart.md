# Quickstart: Agent 프로필과 환경변수 주입 검증

spec의 Success Criteria를 검증하는 절차. 계약은 [contracts/settings-and-run.md](./contracts/settings-and-run.md), 모델 규칙은 [data-model.md](./data-model.md) 참조.

## 사전 준비

```bash
pnpm install
pnpm --filter agentic-workbench tauri dev
```

env 주입 확인용 에이전트 준비: 실행 시 환경변수를 출력하는 명령을 command override로 쓰면 spawn 주입을 직접 확인할 수 있다(예: `sh -c 'env | grep FOO >> /tmp/aw-env-check; exec npx -y @agentclientprotocol/claude-agent-acp'`). 또는 agent에게 "환경변수 FOO 값을 알려줘" 프롬프트로 확인.

## 자동 검증 (회귀 게이트)

```bash
pnpm --filter agentic-workbench check-types
pnpm --filter agentic-workbench test          # command-overrides/profile-invariants/form 모델 테스트 포함
cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml
```

필수 신규 단위 테스트: env normalization(빈/공백 key 제거, value 빈 문자열 유지), 병합 우선순위(profile > global), seed(누락 기본 프로필 복원 + legacy command 매핑), 최소 1개 활성 불변식(위반 저장 거부), 해석 폴백(profile.command → globalCommand → catalog), 오류 메시지에 env value 미포함, runner PATH 결합.

## 시나리오 검증

### S1. 환경변수 주입 (SC-001 / US1)

1. Settings → agent 프로필에서 `claude_code` 기본 프로필에 env `FOO=bar` 저장.
2. 세션 시작에서 그 프로필 선택 → 실행 → agent 프로세스가 `FOO=bar`를 가짐을 확인.
3. global env에 `FOO=global`, `BAZ=1` 저장 후 재실행 → `FOO=bar`(프로필 우선), `BAZ=1`(병합) 확인.
4. env 편집기에 key가 공백뿐인 행을 넣고 저장 → 다시 열면 해당 행 없음(SC-006).
5. 전 과정이 command 문자열 편집 없이 2분 이내(SC-001).

### S2. 프로필 복수 등록·선택 실행 (SC-002 / US2)

1. `claude_code` type으로 커스텀 프로필 "Claude (프록시 경유)"를 다른 command/env로 추가.
2. 세션 시작 목록에 기본/커스텀 두 프로필이 모두 표시.
3. 각각 선택해 실행 → 각자의 command/env로 실행됨(S1의 확인 방법 활용).
4. 커스텀 프로필 삭제 → 목록과 세션 시작 선택지에서 제거.
5. command 미지정 프로필 실행 → catalog 기본 명령으로 동작(FR-007).

### S3. 기본 프로필 안전장치 (SC-004, SC-005 / US3)

1. 설정 데이터가 없는 새 환경(또는 override 설정 초기화) → 기본 프로필 4개 존재.
2. 기본 프로필에 삭제 버튼 없음, command/env 수정 가능.
3. 기본 프로필 3개 disable → 마지막 1개 disable 시도 시 차단 + 안내 문구(SC-005).
4. disable된 프로필이 세션 시작 목록에 없음.
5. (파일 직접 편집으로) 기본 프로필을 지운 저장 데이터 로드 → 자동 seed 복원(FR-008).

### S4. 하위 호환 (SC-003)

1. 구버전 형식(`globalCommand` + `agentCommands`만 있는) override 데이터를 준비(기존 사용자 파일 또는 수동 작성).
2. 앱 실행 → 오류 없이 로드되고, 기본 프로필의 command에 legacy `agentCommands` 값이 반영되어 기존과 동일한 명령으로 실행.
3. env 미설정 프로필 실행 → 기존과 동일 동작(FR-013), 회귀 0건.

### S5. 실행 환경 안전성 (edge cases)

1. env에 `PATH=/custom/bin` 저장 후 실행 → agent가 정상 spawn되고(`npx` 탐색 가능) `/custom/bin`이 PATH 앞에 포함.
2. 실행 중 세션이 있는 상태에서 해당 프로필 disable → 진행 중 세션 영향 없음, 다음 시작 목록에서만 제외.
3. 세션 재사용(reuse) 모드 → 프로필 도입 후에도 기존 provider 세션 목록이 조회·재사용됨.

## 완료 기준 요약

| SC | 확인 방법 | 목표 |
|---|---|---|
| SC-001 | S1 수행 시간 | 2분 이내, 명령 문자열 편집 없음 |
| SC-002 | S2 | 프로필별 구성으로 100% 실행 |
| SC-003 | S4 | 무조치 하위 호환, 회귀 0건 |
| SC-004 | S3-1/3/5 | 기본 4개 상시 존재, 활성 0개 불가 |
| SC-005 | S3-3/4 | 비표시 + 차단 안내 |
| SC-006 | S1-4 | 빈/공백 key 미저장 |
