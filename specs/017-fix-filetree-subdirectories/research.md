# Research: AW Worktree Session Files 하위 디렉터리 조회 수정

## Decision: AW 내부 회귀로 범위를 제한한다

**Rationale**: 명세의 문제 지점은 `apps/agentic-workbench`의 worktree session page Files 섹션이다. 현재 파일트리 UI, worktree-file entity adapter, Tauri 파일 조회 provider가 모두 AW 내부에 존재하므로 다른 앱이나 shared package를 수정하지 않고 회귀를 검증하고 고칠 수 있다.

**Alternatives considered**:
- `packages/*`에 공통 파일트리 모델을 추가: 다른 앱 소비자가 확인되지 않았고 이번 문제는 AW 릴리즈 회귀라 과도하다.
- Git Explorer까지 같은 파일트리 계약으로 확장: 사용자 요청 범위를 벗어난다.

## Decision: lazy loading 계약을 유지하되 상대 경로 보존을 테스트로 고정한다

**Rationale**: Files tab은 루트 직계 목록을 먼저 읽고, 폴더를 펼칠 때 `{ dir, depth: 1 }`로 직계 항목을 추가 조회한다. 이 구조는 큰 worktree에서 초기 렌더 비용을 줄이므로 유지한다. 회귀 수정은 lazy loading을 제거하기보다 `relativePath`가 root 기준 전체 경로로 유지되고, 선택 파일 조회에 그대로 전달되는지 검증해야 한다.

**Alternatives considered**:
- 모든 파일을 한 번에 조회: 구현은 단순하지만 큰 repository에서 성능과 응답 크기 회귀를 만들 수 있다.
- frontend에서 경로를 재조합: backend가 이미 root 기준 `relativePath`를 제공하므로 중복 조합은 경로 손상 위험이 크다.

## Decision: 파일 조회 안전성은 Tauri application/infrastructure 경계에서 유지한다

**Rationale**: `read_worktree_text_file`은 사용자가 선택한 상대 경로로 filesystem을 읽는다. 릴리즈 회귀를 고치더라도 root escape, absolute path, parent-dir path 거부는 유지되어야 한다. Tauri command는 지금처럼 service에 위임하고, 실제 canonicalization과 file read는 provider 경계에서 검증한다.

**Alternatives considered**:
- frontend에서만 경로를 제한: 사용자-visible 오류는 줄일 수 있지만 보안 경계가 될 수 없다.
- Tauri command에 직접 path 검증 추가: hexagonal boundary를 깨고 중복 검증을 만든다.

## Decision: fixture는 사용자 흐름과 path edge case를 함께 커버한다

**Rationale**: 명세의 성공 기준은 릴리즈 사용자 흐름이다. 따라서 테스트/Storybook fixture는 루트 파일, 1단계 하위 파일, 2단계 이상 하위 파일, 같은 basename의 다른 디렉터리 파일, 공백/한글 경로, 읽기 실패 파일을 포함해야 한다. 이 fixture는 frontend 선택 상태와 backend path read 양쪽에서 재사용 가능한 검증 기준이 된다.

**Alternatives considered**:
- backend provider 테스트만 추가: UI 선택 상태와 stale/error rendering 회귀를 놓친다.
- Storybook만 추가: path safety와 릴리즈에서의 Tauri read contract를 보장하지 못한다.

## Decision: 릴리즈 검증은 패키징 전체 대신 릴리즈에 준하는 실행 경로를 우선 문서화한다

**Rationale**: 전체 앱 패키징은 시간이 오래 걸릴 수 있고 환경 의존성이 크다. quickstart는 우선 `pnpm --dir apps/agentic-workbench check-types`, frontend tests, Tauri Rust tests를 기본 검증으로 두고, 구현 단계에서 가능한 경우 릴리즈 빌드 또는 릴리즈에 준하는 Tauri 실행으로 사용자 흐름을 확인하도록 한다.

**Alternatives considered**:
- 매번 전체 installer/package 생성 요구: 회귀 수정마다 비용이 크고 CI/로컬 환경에서 실패 가능성이 높다.
- 개발 서버만 검증: 사용자 요청의 "릴리즈 버전" 조건을 충분히 만족하지 못한다.
