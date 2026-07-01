# 기능 명세: About 메뉴 앱 정보 표시

**기능 브랜치**: `[004-about-menu-app-info]`

**작성일**: 2026-07-01

**상태**: 초안

**입력**: Native menu에 About 메뉴를 추가하고 실행 중인 앱의 버전과 빌드 commit hash를 확인할 수 있게 한다.

## 사용자 시나리오 및 테스트 *(필수)*

### 사용자 스토리 1 - 앱 정보 확인 경로 발견 (우선순위: P1)

사용자는 실행 중인 `agentic-workbench` 앱의 native menu에서 About 메뉴를 찾아 앱 정보를 확인할 수 있다.

**우선순위 근거**: 사용자가 버그 리포트나 배포 확인을 위해 앱 정보를 찾는 첫 진입점이므로 핵심 가치다.

**독립 테스트**: 앱을 실행한 뒤 native menu를 열어 About 항목이 보이는지 확인한다.

**인수 시나리오**:

1. **Given** 사용자가 `agentic-workbench` 앱을 실행 중임, **When** 앱 native menu를 확인함, **Then** About 메뉴 항목을 볼 수 있다.
2. **Given** 사용자가 About 메뉴 항목을 볼 수 있음, **When** 해당 항목을 선택함, **Then** 앱 정보 창 또는 dialog가 열린다.

---

### 사용자 스토리 2 - 버전과 빌드 식별자 확인 (우선순위: P1)

사용자는 About 창에서 현재 실행 중인 앱의 프로그램 버전과 빌드 commit hash를 확인할 수 있다.

**우선순위 근거**: 버전과 commit hash는 버그 재현, 배포 검증, 회귀 추적에 직접 필요한 정보다.

**독립 테스트**: About 메뉴를 선택한 뒤 표시된 정보에 앱 버전과 commit hash가 포함되어 있는지 확인한다.

**인수 시나리오**:

1. **Given** 사용자가 About 창을 열었음, **When** 창 내용을 확인함, **Then** 프로그램 버전이 명확한 라벨과 함께 표시된다.
2. **Given** 사용자가 About 창을 열었음, **When** 창 내용을 확인함, **Then** 빌드 commit hash가 명확한 라벨과 함께 표시된다.

---

### 사용자 스토리 3 - 빌드 식별자 누락 환경에서도 안정적으로 확인 (우선순위: P2)

사용자는 개발 환경이나 특수 빌드 환경에서 commit hash를 확인할 수 없더라도 깨지지 않는 About 창을 볼 수 있다.

**우선순위 근거**: 개발/배포 환경 차이 때문에 정보가 누락되더라도 앱 정보 확인 흐름이 실패하면 안 된다.

**독립 테스트**: commit hash를 사용할 수 없는 환경에서 앱을 실행하고 About 창이 fallback 값을 표시하는지 확인한다.

**인수 시나리오**:

1. **Given** 실행 환경에서 빌드 commit hash를 확인할 수 없음, **When** 사용자가 About 창을 엶, **Then** commit 항목에는 명확한 fallback 값이 표시된다.
2. **Given** fallback 값이 표시되는 About 창이 열려 있음, **When** 사용자가 창을 닫음, **Then** 기존 작업 화면은 정상 상태를 유지한다.

### 예외 상황

- commit hash를 확인할 수 없는 경우에도 About 창은 열려야 하며 commit 항목은 `unknown`처럼 명확한 fallback 값을 표시해야 한다.
- 앱 버전 정보를 읽을 수 없는 예외 상황에서도 About 창의 레이아웃은 깨지지 않아야 하며 명확한 fallback 값을 표시해야 한다.
- About 창은 현재 작업 흐름을 방해하지 않도록 작고 단순해야 한다.
- About 창을 여러 번 열고 닫아도 앱의 현재 화면, 세션, 작업 상태가 변경되지 않아야 한다.
- release build와 dev build에서 표시되는 항목 이름과 순서는 일관되어야 한다.

## 요구사항 *(필수)*

### 기능 요구사항

- **FR-001**: 시스템은 `agentic-workbench` 앱의 native menu에 About 메뉴 항목을 제공해야 한다.
- **FR-002**: 사용자는 About 메뉴 항목을 선택해 작은 앱 정보 창 또는 dialog를 열 수 있어야 한다.
- **FR-003**: 앱 정보 창은 프로그램 버전을 명확한 라벨과 함께 표시해야 한다.
- **FR-004**: 표시되는 프로그램 버전은 앱 배포 단위의 공식 버전 값과 일치해야 한다.
- **FR-005**: 앱 정보 창은 빌드 commit hash를 명확한 라벨과 함께 표시해야 한다.
- **FR-006**: 시스템은 빌드 commit hash를 확인할 수 없는 환경에서 명확한 fallback 값을 표시해야 한다.
- **FR-007**: 앱 정보 창은 정보 확인 목적에 맞게 단순한 내용만 표시해야 하며 사용자의 현재 작업 화면을 변경하지 않아야 한다.
- **FR-008**: 시스템은 dev build와 release build에서 같은 정보 항목, 라벨, 표시 순서를 유지해야 한다.
- **FR-009**: About 메뉴와 앱 정보 창 동작은 수동 검증 절차 또는 동등한 검증 기록으로 확인 가능해야 한다.

### 핵심 개체

- **App Information**: 실행 중인 앱을 식별하기 위한 정보 묶음. 앱 이름, 프로그램 버전, 빌드 commit hash, fallback 표시 값을 포함한다.
- **About Entry Point**: 사용자가 앱 정보를 열기 위해 선택하는 native menu 항목.
- **About Dialog**: 앱 정보를 표시하는 작은 정보 창 또는 dialog.

## 헌법 정렬 *(필수)*

- **Monorepo boundary**: 범위는 `apps/agentic-workbench`에 한정한다. 다른 앱 또는 shared package로 확장하지 않는다.
- **Frontend layering**: 별도 화면 UI가 필요한 경우 앱 조합 계층 또는 앱 정보 전용의 작고 독립적인 표시 단위로 제한한다. 재사용 가능한 cross-app UI 추출은 범위 밖이다.
- **Backend boundary**: 앱 정보 제공과 native menu 연결은 앱 shell 경계에서 다룬다. 도메인 비즈니스 모델이나 영속 저장소 변경은 필요하지 않다.
- **Shared core vs UI**: 현재 요구는 단일 앱의 정보 표시이므로 shared core 또는 shared UI 추출은 의도적으로 피한다.
- **Persistence and safety**: 파일 저장, 사용자 데이터, agent session, 권한 상태는 변경하지 않는다. About 창은 읽기 전용 정보만 표시한다.
- **Documentation and Storybook**: 관련 native menu 동작과 수동 검증 절차를 `docs/*.md`에 한국어로 문서화한다. native menu/dialog는 Storybook 검증 대상이 아니므로 Storybook 추가는 필요하지 않다.

## 성공 기준 *(필수)*

### 측정 가능한 결과

- **SC-001**: 사용자는 앱 실행 후 10초 이내에 native menu에서 About 메뉴를 찾고 열 수 있다.
- **SC-002**: About 창을 연 사용자는 버전과 commit hash 또는 fallback 값을 5초 이내에 확인할 수 있다.
- **SC-003**: commit hash를 사용할 수 없는 환경에서도 About 창 열기 성공률은 100%이며 commit 항목에는 명확한 fallback 값이 표시된다.
- **SC-004**: dev build와 release build 모두에서 About 창은 같은 세 개의 핵심 정보 범주인 앱 이름, 버전, commit 정보를 표시한다.
- **SC-005**: About 창을 열고 닫은 뒤에도 사용자의 현재 작업 화면과 진행 중인 세션 상태는 변경되지 않는다.
- **SC-006**: 완료 전 검증 기록에는 About 메뉴 위치, 표시 정보, commit fallback 동작 확인 절차가 포함된다.

## 가정

- 이번 기능의 대상 앱은 우선 `agentic-workbench`로 한정한다.
- About 창은 별도의 상세 릴리즈 노트나 라이선스 페이지가 아니라 버그 리포트와 배포 확인에 필요한 최소 정보를 제공한다.
- commit hash를 확인할 수 없는 환경의 fallback 문구는 사용자와 개발자가 의미를 바로 이해할 수 있는 짧은 값으로 표시한다.
- 앱 버전은 사용자가 배포 산출물과 비교할 수 있는 공식 앱 버전을 의미한다.
- About 창은 읽기 전용이며 사용자가 값을 복사하는 기능은 이번 범위에 포함하지 않는다.
