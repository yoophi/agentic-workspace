# Implementation Plan: 창 글꼴 크기 조정

**Branch**: `main` (feature branch 미생성) | **Date**: 2026-07-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/035-adjust-font-size/spec.md`

## Summary

Agentic Workbench에 앱 전체 글꼴 크기 환경설정 `-2`~`2`를 추가한다. 백엔드의
`AppearancePreferencesService`가 현재 값의 단일 권위가 되어 JSON 영속화와 범위
정규화를 담당하고, 설정 Slider의 절대값 변경과 `Cmd++`/`Cmd+-`의 상대값 변경을 같은
서비스로 수렴시킨다. 저장 성공 후 모든 WebView에 정규화된 값을 방송하며, 각 창의 앱
Provider는 초기 렌더 전에 값을 적용하고 이후 이벤트를 구독한다.

글꼴 변화는 WebView 확대나 루트 `font-size` 변경이 아니라 Tailwind 4의 텍스트 크기
토큰에 단계당 1px의 오프셋을 적용한다. 이 방식은 텍스트 계층을 유지하면서 spacing,
아이콘, 이미지의 크기를 그대로 둔다. 설정 UI는 기존 shadcn/ui 체계의 Slider를
`components/ui`에 추가하고, 재사용 가능한 `features/font-size-adjustment` 조정
컴포넌트로 구성한다.

## Technical Context

**Language/Version**: TypeScript 5.6+, React 19, Rust edition 2024

**Primary Dependencies**: Tauri 2.11.3, `@tauri-apps/api` 2.x, Tailwind CSS 4.3,
shadcn/ui `radix-nova`, `radix-ui` 1.6, TanStack Query 5, Vitest 4, Serde/serde_json

**Storage**: Tauri app-data의 `appearance-preferences.json`; 기존
`infrastructure/json_store.rs`의 원자적 쓰기와 `.bak` 복구 사용

**Testing**: Vitest 기반 순수 모델·API wrapper·정적 UI 테스트, Rust `cargo test` 기반
domain/application/repository 테스트, Storybook 접근성·시각 상태 검증, 실행 앱의 다중
창 수동 검증

**Target Platform**: macOS Agentic Workbench 데스크톱 앱; 기존 빌드 호환성을 위해
비-macOS 컴파일은 유지하되 이번 단축키 계약은 macOS `Meta` 키가 직접 범위

**Project Type**: Tauri 멀티 WebView 데스크톱 앱

**Performance Goals**: 단축키/Slider 변경 후 모든 열린 AW 창에서 1초 이내 반영; 설정
조회 완료 후 첫 콘텐츠 프레임부터 저장된 단계 적용; 값 변경은 창 수에 선형인 단일
브로드캐스트로 처리

**Constraints**: 값은 정확히 `-2..2`; 경계 증감은 멱등; 저장 성공 전에는 메모리 상태나
창 표시를 확정하지 않음; 글꼴 변경으로 실행 세션·입력·선택·스크롤 컨테이너를 재마운트하지
않음; 이미지·아이콘·spacing은 확대하지 않음; 기존 `Cmd+,` 동작 불변

**Scale/Scope**: `apps/agentic-workbench` 단일 소비자, main/settings/모든 `session-*`
창, 다섯 단계, 앱 설정 JSON 문서 1개, Tauri command 3개와 앱 전체 event 1개

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS — 모든 신규 코드는
  `apps/agentic-workbench`에 둔다. 다른 앱의 확정된 소비 요구가 없어 `packages/*`나
  `crates/*`를 변경하지 않고 앱 간 직접 import도 추가하지 않는다.
- **Feature-Sliced Frontend Architecture**: PASS — 전역 초기화·구독은
  `app/providers`, 환경설정 타입/API는 `entities/appearance-preferences`, 단축키와
  Slider 사용자 동작은 `features/font-size-adjustment`, 설정 화면 조립은
  `pages/settings`, shadcn 생성 Slider는 `components/ui`에 둔다.
- **Hexagonal Tauri Backend Architecture**: PASS — 순수 값은
  `domain/appearance_preferences.rs`, 저장 추상화는 AW가 사용하는 전용
  `ports/appearance_preferences_repository.rs`, 상태 전이·정규화는
  `application/appearance_preferences_service.rs`, JSON은 `infrastructure`, Tauri
  command와 메뉴/앱 이벤트 진입은 `inbound`/`lib.rs`에 둔다. command는 서비스에만
  위임한다.
- **Shared Core Before Shared UI**: PASS — 단일 앱 사용자 경험이므로 공유 승격을 하지
  않는다. Slider primitive도 AW의 기존 `components/ui` 레지스트리에 추가한다.
- **Atomic Cross-App Verification**: N/A — `packages/*`와 `crates/*` 변경이 없다.
  공유 Git/Markdown 컴포넌트는 AW에서 소비되는 모습만 시각 회귀 검증한다.
- **Documentation and Storybook**: PASS — `docs/appearance-font-size.md`를 한국어로
  작성하고, Slider molecule과 Settings page의 `-2/0/2`·긴 콘텐츠 상태를 AW
  Storybook에 등록한다.
- **Testing and Safety**: PASS — 범위 정규화·증감·저장 실패 원자성을 Rust 단위
  테스트로 먼저 고정하고, TypeScript 키 조합 파서·CSS 단계 매핑을 순수 테스트한다.
  app-data 밖 경로를 받지 않으며 저장 경로는 Tauri가 해석한 app-data와 고정 파일명만
  사용한다. 세션·프로젝트·권한 데이터는 읽거나 변경하지 않는다.

**Post-design re-check**: PASS — [research.md](./research.md),
[data-model.md](./data-model.md),
[appearance preferences contract](./contracts/appearance-preferences-contract.md),
[quickstart.md](./quickstart.md)에 위 경계와 검증이 구체화되었고 위반이나 미해결
clarification이 없다.

## Project Structure

### Documentation (this feature)

```text
specs/035-adjust-font-size/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── appearance-preferences-contract.md
├── checklists/
│   └── requirements.md
└── tasks.md                              # /speckit-tasks가 생성
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── app/
│   └── providers/
│       ├── appearance-preferences-provider.tsx
│       └── appearance-preferences-provider.test.ts
├── pages/settings/ui/
│   ├── settings-page.tsx
│   └── settings-page.test.tsx
├── features/font-size-adjustment/
│   ├── model/
│   │   ├── keyboard-shortcut.ts
│   │   └── keyboard-shortcut.test.ts
│   └── ui/
│       ├── font-size-slider.tsx
│       ├── font-size-slider.test.tsx
│       └── font-size-slider.stories.tsx
├── entities/appearance-preferences/
│   ├── api/
│   │   ├── appearance-preferences-repository.ts
│   │   └── appearance-preferences-repository.test.ts
│   └── model/
│       ├── font-size-step.ts
│       ├── font-size-step.test.ts
│       └── types.ts
├── components/ui/
│   └── slider.tsx
├── stories/
│   └── pages.stories.tsx
├── main.tsx
└── index.css

apps/agentic-workbench/src-tauri/src/
├── domain/
│   └── appearance_preferences.rs
├── ports/
│   └── appearance_preferences_repository.rs
├── application/
│   └── appearance_preferences_service.rs
├── inbound/
│   └── tauri_commands.rs
├── infrastructure/
│   └── json_appearance_preferences_repository.rs
└── lib.rs

apps/agentic-workbench/.storybook/mocks/
├── tauri-core.ts
└── tauri-event.ts

docs/
└── appearance-font-size.md
```

**Structure Decision**: 외형 환경설정은 agent 실행 설정의 synthetic working-directory
레코드와 의미·생명주기가 다르므로 별도 도메인과 저장 문서로 둔다. 애플리케이션 서비스는
repository와 `Mutex`로 현재 값을 직렬화해 다중 창의 상대 증감 경쟁을 막는다. 프런트엔드는
각 WebView가 같은 Provider를 하나씩 가지되 백엔드의 canonical 값을 구독하므로 창별
상태가 갈라지지 않는다. 공유 패키지는 수정하지 않는다.

## Implementation Approach

### 1. 백엔드 단일 권위 환경설정

1. `FontSizeStep`은 `-2..2`만 유효하고 기본값은 `0`이다. 명시적 Slider 설정은 유효
   범위 밖 입력을 `0`으로 정규화하고, 상대 증감은 현재 값에서 `-1/+1`만 받아 경계에서
   clamp한다.
2. `AppearancePreferencesService<R>`는 concrete repository와 현재 값의 `Mutex`를
   소유한다. `set`/`adjust`는 잠금 안에서 다음 값을 계산하고 저장에 성공한 뒤에만
   메모리 값을 교체한다. 저장 실패 시 이전 값이 유지된다.
3. 시작 시 repository가 `appearance-preferences.json`을 읽는다. 누락은 `0`, 범위 밖
   값은 `0`으로 정규화해 다시 저장한다. 깨진 JSON은 기존 `.bak` 복구를 먼저 사용하고,
   백업도 실패하면 원본을 진단 가능한 이름으로 보존한 뒤 기본 문서를 원자적으로 쓴다.
4. `get_appearance_preferences`, `set_font_size_step`,
   `adjust_font_size_step` command는 서비스에 위임한다. 변경 command는 영속화가 끝난
   canonical 값을 반환하고 `app://appearance-preferences-changed`를 모든 창에 emit한다.
   경계에서 실제 값이 그대로면 중복 저장/event를 생략해도 되지만 canonical 응답은
   반환한다.

### 2. 초기 적용과 창 간 동기화

1. `AppearancePreferencesProvider`는 모든 route보다 위에 한 번 mount된다.
2. Provider는 event listener를 먼저 등록한 뒤 현재 값을 조회한다. hydration 중 event가
   도착하면 더 최신 event 값을 우선해 초기 조회 응답이 새 값을 덮지 않게 한다.
3. 초기 값이 적용되기 전에는 route 콘텐츠를 렌더하지 않는 짧은 app-shell gate를 두어
   기본 크기에서 저장 크기로 튀는 첫 프레임을 막는다. 조회 실패는 `0`으로 렌더를 계속하고
   설정 화면에서 재시도 가능한 오류를 제공한다.
4. canonical 값은 `document.documentElement.dataset.fontSizeStep`에 반영한다. event
   업데이트는 Provider state와 dataset만 바꾸며 route tree key를 변경하거나 세션
   컴포넌트를 재마운트하지 않는다.

### 3. 단축키

1. 각 창의 Provider 또는 전용 controller가 capture 단계 `keydown` listener를 한 번
   등록한다.
2. macOS에서 `metaKey=true`, `ctrlKey=false`, `altKey=false`이고 logical
   `event.key`가 `+` 또는 `=`이면 `+1`, `-`이면 `-1`로 해석한다. `=` 허용은 일반적인
   `Shift+=`/키보드 배열 차이를 흡수한다.
3. 인식한 조합에서만 `preventDefault()`하고 backend `adjust` command를 호출한다.
   입력 요소 안에서도 동작하지만 `Cmd+,`, ctrl-only, alt 조합, 일반 입력은 건드리지
   않는다.
4. Tauri native `CmdOrCtrl++`는 현재 physical accelerator parser가 literal `+`를
   안정적으로 표현하지 못하므로 사용하지 않는다. 기존 Preferences `CmdOrCtrl+,`는
   `lib.rs` 회귀 테스트로 보존한다.

### 4. 글꼴 토큰과 레이아웃

1. 단계 매핑은 `-2=-2px`, `-1=-1px`, `0=0px`, `1=+1px`, `2=+2px`다.
2. `index.css`의 Tailwind typography 변수 `--text-xs`부터 사용 중인 display 단계까지
   각각 기존 기준값에 `--aw-font-size-offset`을 더한다. 기본 unitless line-height
   비율은 변경된 크기에 따라 함께 계산되며, 명시적 `leading-*`은 다섯 단계에서 clipping
   여부를 검증한다.
3. `html`의 root font-size, Tailwind `--spacing`, WebView zoom은 바꾸지 않는다.
   따라서 rem 기반 padding/아이콘과 이미지 크기는 유지된다.
4. AW 로컬의 임의 `text-[…]` 크기는 의미에 맞는 동적 typography token으로 옮긴다.
   공유 Git/Markdown 패키지의 임의 크기는 패키지 자체를 AW 요구 때문에 바꾸지 않고,
   AW에서 사용자 핵심 텍스트가 단계 적용을 받는지 검증한 뒤 필요한 경우 앱 경계의
   scoped 호환 token으로 보완한다.

### 5. 설정 Slider와 Storybook

1. shadcn CLI로 현재 `radix-nova` preset의 Slider를
   `components/ui/slider.tsx`에 추가한다.
2. `FontSizeSlider`는 `Field` 조합 안에서 `min=-2`, `max=2`, `step=1`, 단일 thumb,
   현재 signed value, 다섯 tick label, `aria-valuetext`를 제공한다. 별도 Save 버튼 없이
   값 변경 시 canonical set command를 호출한다.
3. pending 동안 중복 set을 제어하고, 저장 실패 시 기존 canonical 값으로 되돌린 뒤
   사용자가 다시 시도할 수 있는 오류를 표시한다.
4. 설정 페이지 상단에 Appearance 섹션을 두고 기존 agent profile 설정과 독립적으로
   loading/error 상태를 다룬다. Slider molecule과 `-2/0/2`, 긴 콘텐츠 Settings page를
   Storybook에 등록한다.

## Complexity Tracking

> 위반 없음 — 해당 없음.
