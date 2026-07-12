# Requirements Quality Checklist: Markdown 작업 보드

**Purpose**: [[speckit-spec | 기능 명세]]의 완전성과 검증 가능성을 구현 전에 확인한다.  
**Plan**: [[speckit-plan]] | **Tasks**: [[speckit-tasks]]

## Requirement Completeness

- [x] CHK001 완료와 미완료 상태가 명시되어 있는가?
- [x] CHK002 H1 chapter 범위가 정의되어 있는가?
- [ ] CHK003 파일 삭제 중 발생하는 동시성 동작이 정의되어 있는가?
- [ ] CHK004 대규모 문서의 성능 기준이 수치로 정의되어 있는가?

## Requirement Clarity

- [x] CHK005 “진행률” 계산식이 모호하지 않은가?
- [x] CHK006 코드 블록의 task 유사 문자열 처리 방식이 명확한가?
- [ ] CHK007 중첩 task의 chapter 소속 규칙이 명시되어 있는가?

## Scenario Coverage

- [x] CHK008 task가 없는 문서의 기대 결과가 정의되어 있는가?
- [ ] CHK009 읽기 실패와 권한 오류 시나리오가 포함되어 있는가?
- [ ] CHK010 문서가 외부에서 변경되는 시나리오가 포함되어 있는가?

## Notes

미완료 항목을 해결한 뒤 `$speckit-plan`과 `$speckit-tasks` 산출물을 다시 검토한다.

