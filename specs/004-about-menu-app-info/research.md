# Research: About 메뉴 앱 정보 표시

## Decision: `agentic-workbench` 단일 앱을 대상으로 한다

**Rationale**: 명세가 우선 대상 앱을 `agentic-workbench`로 정의했고, 현재 요구는 app shell의 native menu와 정보 dialog에 한정된다. 다른 앱에 확장하면 cross-app 공통 설계와 메뉴 정책 결정이 필요해 범위가 커진다.

**Alternatives considered**:

- 모든 Tauri 앱에 동시에 적용: 공통 정책과 shared abstraction이 필요하지만, 현재 사용자 가치에는 과하다.
- shared package/crate로 앱 정보 모델 추출: 두 개 이상의 앱이 소비하지 않으므로 헌법의 shared code 원칙에 맞지 않는다.

## Decision: 앱 버전은 `apps/agentic-workbench/package.json`의 공식 버전 값을 기준으로 한다

**Rationale**: 사용자가 배포 산출물과 비교하는 앱 버전은 frontend package의 `version` 값으로 명시되어 있다. 명세도 이 값을 기준으로 요구한다. Tauri config 또는 Cargo package version과 값이 같더라도 단일 기준은 `package.json`으로 고정해야 회귀 추적이 명확하다.

**Alternatives considered**:

- Tauri config version 사용: 배포 설정과 가깝지만 명세의 기준과 다르다.
- Cargo package version 사용: Rust crate 버전과 앱 버전이 항상 같은 정책인지 보장되지 않는다.

## Decision: build-time metadata로 commit hash를 주입하고 환경 변수 우선, Git 조회 fallback을 사용한다

**Rationale**: release build에서는 CI 환경 변수가 가장 신뢰 가능한 빌드 원천이고, 로컬 개발 빌드에서는 Git checkout에서 commit을 조회하는 방식이 편리하다. 둘 다 실패할 수 있으므로 정보 모델은 fallback 값을 허용해야 한다.

**Alternatives considered**:

- runtime Git 조회: 배포 앱에서 Git metadata가 없을 수 있고, 실행 시 불필요한 외부 프로세스 의존이 생긴다.
- frontend 환경 변수만 사용: native menu/dialog에서 직접 사용하는 app shell 정보와 경계가 어긋날 수 있다.
- Tauri build metadata만 사용: commit hash의 표준 제공 경로가 명세 요구를 완전히 충족하지 않는다.

## Decision: About 창은 작은 native message dialog로 제공한다

**Rationale**: 요구 정보가 앱 이름, 버전, commit hash뿐이므로 별도 webview 창은 과하다. native dialog는 현재 작업 화면을 바꾸지 않고 작은 정보 확인 흐름을 제공한다.

**Alternatives considered**:

- 별도 webview About window: 스타일링 자유도는 높지만 라우팅, 창 생명주기, frontend 상태 영향 검증이 늘어난다.
- 기존 앱 화면 내부 dialog: native menu에서 호출되는 shell 정보 표시에는 불필요하게 frontend coupling이 생긴다.

## Decision: 메뉴 위치는 플랫폼 관례를 따른다

**Rationale**: macOS 사용자는 앱 메뉴에서 About 항목을 기대하고, Windows/Linux 사용자는 Help 메뉴 아래 About 항목을 기대한다. 사용자 발견 가능성과 OS 관례를 우선한다.

**Alternatives considered**:

- 모든 플랫폼에서 Help 메뉴만 사용: macOS 앱 메뉴 관례와 맞지 않는다.
- 별도 App Info 메뉴 생성: 메뉴가 늘어나고 일반적인 About 위치와 다르다.

## Decision: fallback 값은 명확한 짧은 문자열로 표시한다

**Rationale**: commit hash가 없을 때 정보가 비어 있으면 화면 깨짐이나 누락으로 보인다. `unknown` 같은 짧은 값은 버그 리포트에서 "해시 확인 불가" 상태를 명확히 전달한다.

**Alternatives considered**:

- commit 줄 숨김: 사용자가 값이 누락된 것인지 기능이 없는 것인지 알기 어렵다.
- 긴 설명문 표시: 작은 About 창의 목적과 맞지 않고 빠른 확인성을 떨어뜨린다.

## Decision: 검증은 Rust 컴파일 검사와 수동 native menu 확인으로 구성한다

**Rationale**: native OS menu와 dialog는 headless unit test보다 실제 앱 실행 확인이 중요하다. 컴파일 검사는 Tauri menu/dialog API 사용을 검증하고, 수동 절차는 플랫폼별 메뉴 위치와 표시 내용을 확인한다.

**Alternatives considered**:

- frontend typecheck만 수행: native shell 변경 검증에 충분하지 않다.
- 자동 UI 테스트만 요구: native menu 접근 자동화가 환경별로 취약해 계획 단계의 기본 검증으로 과하다.
