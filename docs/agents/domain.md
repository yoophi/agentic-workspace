# 도메인 문서

이 저장소는 multi-context 도메인 문서 구조를 사용한다.

## 코드 탐색 전에 읽을 문서

1. 루트 `CONTEXT-MAP.md`
2. 작업 대상 앱 또는 공유 영역의 `CONTEXT.md`
3. 시스템 공통 결정인 `docs/adr/`
4. 대상 컨텍스트 내부의 `docs/adr/`

문서가 아직 없으면 별도 경고 없이 작업을 계속한다. 도메인 용어나 결정이 실제로 정리될 때 관련 스킬이 문서를 생성한다.

## 구조

- `CONTEXT-MAP.md`: 컨텍스트 목록과 관련 문서 경로
- `docs/adr/`: 워크스페이스 전체에 적용되는 결정
- `apps/<app>/CONTEXT.md`: 앱별 도메인 용어와 경계
- `apps/<app>/docs/adr/`: 앱별 결정
- `packages/<package>/CONTEXT.md`: 독립적인 공유 도메인을 가진 패키지의 용어와 경계
- `packages/<package>/docs/adr/`: 패키지별 결정
- `crates/<crate>/CONTEXT.md`: 독립적인 공유 도메인을 가진 Rust crate의 용어와 경계
- `crates/<crate>/docs/adr/`: crate별 결정

모든 패키지와 crate에 문서를 미리 만들 필요는 없다. 독립적인 도메인 경계가 확인된 영역에만 생성한다.

## 소비 규칙

- 이슈 제목, 리팩터링 제안, 가설, 테스트 이름에는 `CONTEXT.md`의 표준 용어를 사용한다.
- glossary가 피하도록 지정한 동의어를 새로 도입하지 않는다.
- 필요한 개념이 glossary에 없으면 용어 혼선 또는 문서 공백으로 기록한다.
- 기존 ADR과 충돌하는 제안은 해당 ADR을 명시하고 충돌 이유를 드러낸다.
