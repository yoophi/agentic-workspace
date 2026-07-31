# Quickstart: 창 글꼴 크기 조정 검증

## 사전 조건

- 저장소 루트에서 `pnpm install` 완료
- macOS에서 Agentic Workbench를 실행할 수 있는 Tauri 개발 환경
- 현재 feature 문서: `specs/035-adjust-font-size/`

## 자동화 검증

### 프런트엔드

```bash
pnpm --filter @yoophi/agentic-workbench check-types
pnpm --filter @yoophi/agentic-workbench test
pnpm --filter @yoophi/agentic-workbench build-storybook
```

기대 결과:

- `FontSizeStep` 정규화와 `-2..2` 경계 증감 테스트 통과
- `Meta +`, `Meta =`, `Meta -`는 처리하고 `Cmd+,`/ctrl-only/alt 조합은 무시
- appearance API wrapper가 세 command의 정확한 payload를 전송
- Provider hydrate/event/cleanup과 같은 값 event의 멱등 처리 통과
- Slider가 다섯 단계, 현재 값, label과 키보드 접근성 계약을 렌더
- 기존 AW 테스트와 Storybook build 무손상

### Tauri 백엔드

```bash
cargo test -p agentic-workbench
cargo check -p agentic-workbench
```

기대 결과:

- domain의 다섯 값, invalid-to-zero, clamp 테스트 통과
- service의 저장-before-state, 저장 실패 rollback, 빠른 상대 증감 직렬화 테스트 통과
- repository의 누락/roundtrip/backup/손상 복구 테스트 통과
- 기존 Preferences menu ID와 `CmdOrCtrl+,` 회귀 테스트 통과

## Storybook 검증

```bash
pnpm --filter @yoophi/agentic-workbench storybook
```

다음을 확인한다.

1. `Font Size Slider` molecule에서 `-2`, `0`, `2` 상태와 방향키 조작
2. Settings page의 기본/로딩/오류/긴 콘텐츠 상태에서 Slider label·tick·현재 값
3. addon-a11y 결과에 Slider name/value/range 관련 violation이 없음

## 실행 앱 엔드투엔드 검증

```bash
pnpm run tauri:dev:workbench
```

### 시나리오 A — 단축키와 경계

1. 기본 `0`에서 `Cmd++`를 두 번 눌러 `2`가 되는지 확인한다.
2. `Cmd++`를 다섯 번 더 눌러도 `2`이고 오류가 없는지 확인한다.
3. `Cmd+-`를 네 번 눌러 `-2`가 되는지 확인한다.
4. 텍스트 입력 중 같은 동작을 반복하고 입력 내용·focus·선택이 유지되는지 확인한다.
5. `Cmd+,`가 기존처럼 Settings 창을 열고 font step을 바꾸지 않는지 확인한다.

### 시나리오 B — Slider와 창 간 동기화

1. main, settings, 서로 다른 두 `session-*` 창을 연다.
2. Settings Slider를 `+1`로 옮긴 뒤 네 창이 1초 안에 같은 크기로 바뀌는지 확인한다.
3. session 창에서 `Cmd+-`를 누르고 Settings Slider가 `0`으로 갱신되는지 확인한다.
4. 다른 session 창을 새로 열어 첫 콘텐츠 프레임부터 `0`이 적용되는지 확인한다.
5. 빠르게 증가/감소를 번갈아 실행해 마지막 canonical 값이 모든 창에서 같은지 확인한다.

### 시나리오 C — 재실행과 실패 안전성

1. `+2`를 선택하고 앱을 완전히 종료한 뒤 다시 실행한다.
2. 첫 창이 기본 크기로 번쩍였다 바뀌지 않고 처음부터 `+2`로 보이는지 확인한다.
3. repository 자동 테스트로 범위 밖 값, 깨진 현재 JSON, 유효 backup, backup도 깨진 경우가
   각각 `0` 또는 backup 값으로 복구되는지 확인한다.
4. 저장 실패 fixture에서 이전 값 유지, event 미발행, 재시도 가능한 오류 표시를 확인한다.

### 시나리오 D — 5단계 시각 회귀

각 `-2`, `-1`, `0`, `1`, `2`에서 다음 화면을 확인한다.

- dashboard와 project table
- agent timeline, prompt composer, permission dialog, minimap
- workspace의 Markdown, 표, code, diff, file tree
- Settings Slider와 agent profile editor

완료 기준:

- 핵심 텍스트가 잘리거나 겹치지 않고 모든 조작 요소에 접근 가능
- 스크롤 영역은 필요할 때 스크롤로 대응하고 작업 surface가 사라지지 않음
- 대표 icon과 image의 computed width/height가 `0`과 `-2/2`에서 동일
- 현재 session, 입력, 선택, 실행 상태가 글꼴 변경 전후 유지

## 계약 참조

- [데이터 모델](./data-model.md)
- [외형 환경설정 인터페이스 계약](./contracts/appearance-preferences-contract.md)
- [기능 명세](./spec.md)
