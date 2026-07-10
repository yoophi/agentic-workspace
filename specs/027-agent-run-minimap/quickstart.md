# Quickstart: Agent Run 히스토리 미니맵 검증

## 목적

구현 후 명세와 [UI 계약](./contracts/agent-run-minimap-ui.md)을 end-to-end로 검증하는 실행 가이드다.

## 사전 조건

- 저장소 루트에서 pnpm 의존성이 설치되어 있어야 한다.
- `027-agent-run-minimap` 브랜치가 checkout되어 있어야 한다.
- agentic-workbench Storybook용 Tauri mock을 사용할 수 있어야 한다.

## 자동 검증

```bash
pnpm --filter @yoophi/agentic-workbench check-types
pnpm --filter @yoophi/agentic-workbench test
pnpm --filter @yoophi/agentic-workbench build
pnpm --filter @yoophi/agentic-workbench build-storybook
```

**Expected**:

- 타입 오류가 없다.
- entry 투영, 요약/크기 제한, 좌표 변환, 양끝 clamp, 키보드 이동, filter pending seek, panel 격리 테스트가 통과한다.
- 프로덕션 앱과 Storybook 정적 빌드가 성공한다.

## Storybook 실행

```bash
pnpm --filter @yoophi/agentic-workbench storybook
```

기본 주소 `http://localhost:6006`에서 Agent Run Minimap organism 상태를 연다.

## 시나리오 1: 긴 대화 탐색

1. 사용자/agent 항목이 20회 이상인 long history 상태를 연다.
2. 미니맵 indicator를 시작, 중간, 끝으로 드래그한다.
3. history에 대응하는 prompt와 agent 출력이 표시되는지 확인한다.
4. history를 휠/트랙패드로 이동해 indicator가 반대로 동기화되는지 확인한다.

**Expected**: 드래그는 rail 밖에서도 capture가 유지되고 양끝에서 clamp된다. 측정 완료 후 indicator와 실제 history 위치 차이는 전체 범위의 5% 이내다.

## 시나리오 2: 키보드 및 접근성

1. indicator에 focus한다.
2. ArrowUp/Down, PageUp/Down, Home, End를 차례로 사용한다.
3. 접근성 패널과 addon-a11y 결과를 확인한다.

**Expected**: 모든 키가 계약된 방향과 단위로 이동하며 slider 이름, 방향, 최소/최대/현재 값이 노출된다. 토글은 아이콘 tooltip과 현재 표시 상태를 제공하고 새 접근성 위반이 없다.

## 시나리오 3: 표시 토글과 좁은 화면

1. 중간 history 위치에서 미니맵을 숨긴다.
2. history가 우측 공간을 회수했는지 확인하고 다시 표시한다.
3. Storybook viewport를 agent panel 최소 폭 360px로 줄인다.

**Expected**: 숨김/표시 전후 scroll 위치가 유지된다. 토글, history, minimap이 겹치거나 버튼/텍스트 크기를 바꾸지 않는다.

## 시나리오 4: 필터 연동

1. Tool 또는 Lifecycle 필터를 선택한다.
2. 미니맵에서 과거 대화 위치 탐색을 시작한다.

**Expected**: 타임라인이 All 필터로 전환되고 최신 layout snapshot이 준비된 뒤 요청 위치가 한 번 적용된다. 목표 prompt/agent 출력이 실제 history에 보인다.

## 시나리오 5: 스트리밍과 패널 격리

1. 최신 출력 위치에서 streaming fixture를 진행한다.
2. 미니맵으로 과거 위치로 이동한 후 출력을 더 추가한다.
3. 미니맵 표시 상태와 scroll 위치가 다른 Main/Extra panel로 전환한다.

**Expected**: 최신 위치에서는 새 출력을 따라가고 과거 위치에서는 사용자의 위치를 유지한다. panel 전환 후 각 panel의 visibility, entries, indicator, scroll 위치가 섞이지 않는다.

## 시나리오 6: 빈 상태와 500항목 성능

1. 빈 history와 한 화면 이하 short history 상태를 각각 연다.
2. 500개 대화 항목 상태에서 indicator를 연속 드래그한다.

**Expected**: 빈 상태에서는 seek가 시작되지 않고 short 상태에서는 indicator가 전체 rail을 나타낸다. 500항목 상태에서도 원본 Markdown을 중복 렌더링하지 않으며 조작 후 갱신이 100ms 이내다.

## 완료 판정

- 위 여섯 시나리오가 desktop 및 360px agent panel 폭에서 통과한다.
- [데이터 모델](./data-model.md)의 상태 전이와 [UI 계약](./contracts/agent-run-minimap-ui.md)의 필수 이벤트가 구현과 일치한다.
- 기존 prompt 입력, 출력 streaming, 최신 출력 자동 추적, ACP 필터, 추가 agent panel 전환에 회귀가 없다.

## 구현 검증 기록

**검증일**: 2026-07-10

### 자동 검증

| 항목 | 결과 | 증거 |
|------|------|------|
| TypeScript | PASS | `pnpm --filter @yoophi/agentic-workbench check-types` |
| 전체 Vitest | PASS | 39 files, 258 tests |
| 프로덕션 build | PASS | Vite production build 완료 |
| Storybook build | PASS | 6개 minimap organism story를 포함한 정적 build 완료 |
| 500-entry projection | PASS | tool/lifecycle 제외와 100ms 예산 단위 테스트 통과 |
| 500 viewport revisions | PASS | 5% 정합성 helper와 100ms 예산 단위 테스트 통과 |

### Storybook 등록 확인

다음 story가 Storybook index에 등록되고 각 iframe이 HTTP 200으로 응답함을 확인했다.

- Long History
- Large History (500 entries)
- Empty And Short
- Streaming
- Visibility And Panel Isolation
- Narrow (360px)

### 브라우저 및 네이티브 앱 검증

Chromium Playwright와 현재 worktree에서 다시 시작한 Tauri 개발 앱으로 시나리오 1~6을 검증했다.

| 시나리오 | 결과 | 측정 및 확인 내용 |
|----------|------|-------------------|
| 긴 대화 탐색 | PASS | 20개 prompt/response 흐름의 모든 사용자 prompt가 구분 가능한 항목으로 투영됐고, pointer drag와 키보드로 시작·중간·끝에 도달했다. |
| 키보드 및 접근성 | PASS | Home/End가 0%/100%, ArrowDown이 5% 단위로 이동했다. Tauri 앱에서 vertical slider의 label, min/max/now가 노출됨을 확인했다. |
| 표시 토글과 좁은 화면 | PASS | history 폭이 574px→686px→574px로 변했고 논리 위치는 19%로 유지됐다. 360px viewport에서 `scrollWidth=clientWidth=360`이며 112px rail과 slider가 경계 안에 있었다. |
| 필터 연동 | PASS | User 필터에서 slider 키보드 탐색 시 All 필터로 전환되고 pending seek가 적용됐다. |
| 스트리밍과 패널 격리 | PASS | 끝 위치에서 20→22 항목 추가 후 100%를 유지했다. 과거 위치에서는 `scrollTop=740`을 유지하고 indicator만 53%→45%로 갱신됐다. 한 panel 토글이 다른 panel의 초기 hidden 상태를 바꾸지 않았다. |
| 빈 상태와 500항목 | PASS | 빈 상태 label과 disabled slider, 짧은 기록의 full indicator, 500개 minimap entry 렌더링을 확인했다. 1440×900 Chromium에서 1회 예열(142.4ms) 후 5회 pointer 입력→100% 위치 반영 시간은 34.0/26.3/32.6/28.4/29.1ms, 중앙값 29.1ms였다. |

axe-core 4.12.1을 long/narrow iframe에 적용한 결과 minimap slider, rail, toggle을 대상으로 한 위반은 없었다. 전체 story에는 기존 ACP filter `tablist`, agent selector 이름, 기존 avatar/action 색상 대비에서 3개 유형의 위반이 남아 있으며 이번 minimap 변경 범위 밖의 기존 항목으로 기록한다.

### 요구사항 추적

| 요구사항 | 구현 및 검증 |
|----------|--------------|
| FR-001 | `AgentRunPanel`의 history/minimap 좌우 flex layout |
| FR-002~FR-003 | `entities/agent-run/model/minimap.ts`의 user/assistant projection, summary 및 weight 테스트 |
| FR-004 | 공유 `TimelineLayoutSnapshot`과 viewport indicator 계산 테스트 |
| FR-005~FR-007 | pointer/keyboard seek, scroller offset clamp, passive scroll snapshot 갱신 |
| FR-008~FR-009 | controlled streaming fixture와 기존 48px bottom-stick 정책 회귀 테스트 |
| FR-010~FR-012 | icon tooltip toggle, conditional rail, stable `timelineScrollRef` 계약 테스트 |
| FR-013 | panel-local state와 Main/Extra isolation Storybook 상태 |
| FR-014~FR-015 | vertical slider ARIA 계약과 Arrow/Page/Home/End 계산 테스트 |
| FR-016 | empty, short, long, 500-entry, streaming, hidden, narrow 상태와 테스트 |

### 아키텍처 감사

- 변경은 `apps/agentic-workbench/src/entities/agent-run`, `features/agent-run`, `shared/storybook`, `stories`에 한정된다.
- Tauri/Rust backend, 파일 접근, persistence, session repository는 변경하지 않았다.
- `packages/*`, `crates/*`, 다른 app import를 추가하지 않았다.
- 순수 projection과 interaction 계산은 UI에서 분리하고 Vitest로 검증했다.
