# Phase 0 Research: 창 글꼴 크기 조정

## R1. 환경설정 소유권과 저장 위치

- **Decision**: AW 전용 `AppearancePreferences` 도메인과
  `appearance-preferences.json` 저장소를 신설하고, 백엔드 application service를 현재
  값의 단일 권위로 사용한다.
- **Rationale**: 기존 `AgentRunSettings`는 working directory별 실행 설정이고 설정 창은
  command override를 위해 synthetic key를 사용한다. 앱 전체 외형 값까지 여기에 넣으면
  별개 생명주기가 결합되고 unrelated field overwrite 위험이 생긴다. AW의 기존 JSON
  repository, app-data 경로, 원자적 쓰기/backup 복구 패턴은 그대로 재사용할 수 있다.
- **Alternatives considered**:
  - 기존 agent-run settings에 필드 추가 — 도메인과 scope가 달라 기각.
  - `localStorage`/`BroadcastChannel` — 현재 저장 관례가 아니고 여러 WKWebView의
    동기화·재실행 단일 권위를 보장하기 어려워 기각.
  - 공유 package/crate — 소비 앱이 하나뿐이라 constitution의 공유 기준을 충족하지 않아
    기각.

## R2. `Cmd++`/`Cmd+-` 인식 방식

- **Decision**: 모든 AW WebView에 capture 단계 `keydown` listener를 한 번 등록하고
  macOS `metaKey`와 logical `event.key` (`+`/`=`/`-`)를 해석한다. 인식된 조합만 backend
  상대 증감 command로 전달한다.
- **Rationale**: 현재 Tauri 2.11.3 `MenuItem::with_id`는 문자열을 muda의 physical
  `Accelerator`로 parse하며 literal `+`는 안정적으로 표현되지 않는다. `Shift+Equal`은
  미국식 배열에 종속된다. logical key 이벤트는 실제 사용자가 만든 `+`를 기준으로 하므로
  입력 요소와 키보드 배열 edge case를 더 잘 다룬다. `Cmd+,`는 별도 key라 충돌하지 않는다.
- **Alternatives considered**:
  - `CmdOrCtrl++` native accelerator — parse 실패가 조용히 accelerator 제거로 이어져
    기각.
  - `CmdOrCtrl+Shift+Equal` — 배열 종속성이 있어 기각.
  - global shortcut plugin — 앱이 비활성일 때도 단축키를 가로채 범위가 과하고 새
    dependency가 필요해 기각.

## R3. 다중 창 동기화와 경쟁 조건

- **Decision**: `AppearancePreferencesService`가 repository와 in-memory 값을
  `Mutex`로 직렬화한다. set/adjust는 저장 성공 후 값을 교체하고, inbound adapter가
  canonical 값 하나를 app-wide event로 방송한다. 각 창은 listener를 먼저 설치한 뒤
  hydrate하며 hydration 중 받은 event를 우선한다.
- **Rationale**: 창별 read-modify-write는 빠른 교차 입력에서 update loss를 만들 수
  있다. 백엔드 상대 증감과 잠금은 한 번에 하나의 canonical transition만 허용한다.
  Tauri `AppHandle::emit`은 모든 target에 event를 전달하는 기존 인프라와 맞는다.
- **Alternatives considered**:
  - 각 창 TanStack Query cache만 무효화 — cache가 WebView 사이에 공유되지 않아 기각.
  - 저장 후 polling — 1초 반영 목표에 비효율적이며 일관성이 늦어 기각.
  - optimistic UI — 저장 실패 rollback과 창별 divergence가 복잡해져, 저장 후 반영을
    선택.

## R4. 첫 프레임과 상태 보존

- **Decision**: Provider가 저장값을 hydrate하고 document typography state를 적용할
  때까지 route child를 짧게 gate한다. 이후 변경은 root dataset/CSS variable만 바꾸고
  route/component key는 유지한다.
- **Rationale**: 저장값이 `2`인데 기본 `0`으로 먼저 렌더되면 재실행 직후 눈에 띄는
  크기 flash가 생긴다. 반대로 route tree를 다시 만들면 입력·선택·세션 상태가 유실될 수
  있다. 초기 gate와 CSS-only update가 두 요구를 함께 만족한다.
- **Alternatives considered**:
  - 렌더 후 비동기 적용 — 초기 flash 때문에 기각.
  - 값 변경마다 App remount — FR-010 위반으로 기각.

## R5. 글꼴 크기 적용 방식과 단계 간격

- **Decision**: Tailwind의 text token 기준값에 단계당 1px 오프셋을 더한다
  (`-2,-1,0,+1,+2px`). root font-size, `--spacing`, WebView zoom은 변경하지 않는다.
- **Rationale**: AW와 공유 UI의 대다수 텍스트가 `text-xs`~`text-3xl` token을 사용하고
  아이콘 크기는 독립된 spacing token을 사용한다. 1px 간격은 작은 텍스트에서도 눈에
  띄면서 `-2`/`2`의 레이아웃 충격이 제한적이다. 텍스트 token만 바꾸면 이미지·아이콘
  자체 배율이 보존된다.
- **Alternatives considered**:
  - `html { font-size }` — rem 기반 spacing/아이콘까지 확대되어 FR-012와 충돌.
  - WebView zoom/CSS zoom — 전체 화면 배율을 바꿔 기각.
  - 백분율 비례 확대 — 큰 heading 변화가 과도하고 검증 기준이 복잡해져 1px step을 선택.

## R6. Slider 구성과 접근성

- **Decision**: shadcn registry의 Radix Slider를 `components/ui`에 추가하고
  `Field`로 label/description/error를 조합한다. `min=-2`, `max=2`, `step=1`의 controlled
  단일 thumb와 현재 signed 값, 다섯 tick을 제공한다.
- **Rationale**: 프로젝트는 `radix-nova`, `rsc=false`, Tailwind 4이며 `Field`는 이미
  설치되어 있다. Slider는 키보드 방향키와 range semantics를 제공하므로 custom range
  구현보다 접근성·일관성이 높다.
- **Alternatives considered**:
  - 5개 Button/ToggleGroup — 사용자가 Slider를 명시했고 연속 축의 단계라는 의미가
    약해져 기각.
  - native `<input type="range">` 직접 스타일링 — 기존 design system을 우회해 기각.

## R7. 손상값과 저장 실패

- **Decision**: 누락은 `0`, parse 가능한 범위 밖 값은 `0`으로 canonical rewrite한다.
  깨진 JSON은 공용 `.bak` 복구를 먼저 시도하고, 백업까지 실패하면 손상 파일을 보존한 뒤
  기본 문서를 원자적으로 생성한다. runtime 저장 실패는 이전 in-memory 값을 유지하고
  event를 방송하지 않는다.
- **Rationale**: 앱은 외형 설정 하나 때문에 시작이 막히면 안 되지만 진단 정보도 지우면
  안 된다. 저장-before-state/event 순서는 모든 창이 실제 영속 상태에만 수렴하게 한다.
- **Alternatives considered**:
  - 모든 read error를 그대로 사용자에게 전파해 렌더 중단 — FR-011 위반.
  - 오류 상태에서도 optimistic event — 재실행 값과 현재 창 값이 달라져 기각.

## R8. 검증 전략과 임의 글꼴 크기

- **Decision**: Rust에서 domain/application/repository를 test-first로 고정하고,
  TypeScript에서 shortcut parser·step mapping·API contract를 순수 테스트한다. Slider와
  settings 조합은 정적 마크업/Storybook, 실제 key/event·다중 창·layout은 실행 앱에서
  검증한다. AW 로컬 arbitrary text size는 동적 token으로 옮기고 공유 package는 수정하지
  않은 채 AW 경계에서 시각 검증한다.
- **Rationale**: 현재 AW Vitest는 Node/SSR 중심이라 WebView layout과 실제 키보드
  modifier를 완전히 재현하지 않는다. 순수 규칙은 빠른 자동 테스트로, OS/WebView 통합은
  quickstart의 재현 가능한 수동 시나리오로 나누는 것이 기존 관례에 맞는다. 단일 앱 요구로
  공유 package를 바꾸면 다른 소비 앱에 예기치 않은 typography 변경이 생긴다.
- **Alternatives considered**:
  - jsdom/브라우저 test dependency를 이번 기능만 위해 도입 — 비용 대비 범위가 커 기각.
  - shared UI의 모든 absolute size 변경 — cross-app 요구와 검증 없이 constitution
    경계를 넓혀 기각.

## 미해결 항목

- 없음. 모든 기술 선택과 clarification을 해결했다.
