# 이슈 트래커: GitHub

이 저장소의 이슈와 PRD는 GitHub Issues에서 관리하며 모든 작업에는 `gh` CLI를 사용한다.

## 기본 작업

- 생성: `gh issue create --title "..." --body "..."`
- 조회: `gh issue view <number> --comments`
- 목록: `gh issue list --state open --json number,title,body,labels,comments`
- 댓글: `gh issue comment <number> --body "..."`
- 라벨 추가·삭제: `gh issue edit <number> --add-label "..."` 또는 `--remove-label "..."`
- 종료: `gh issue close <number> --comment "..."`

저장소는 현재 Git remote에서 자동으로 식별한다.

## Pull request를 triage 요청으로 취급할지 여부

**PRs as a request surface: no.**

## 스킬 연동

- “이슈 트래커에 발행”은 GitHub Issue 생성을 뜻한다.
- “관련 티켓 조회”는 `gh issue view <number> --comments`를 뜻한다.
- bare issue 번호가 PR인지 이슈인지 불명확하면 `gh pr view` 후 `gh issue view`를 시도한다.

## Wayfinder

- Map은 `wayfinder:map` 라벨을 가진 단일 이슈다.
- Child ticket은 GitHub sub-issue로 연결하고 `wayfinder:<type>` 라벨을 사용한다.
- 차단 관계는 GitHub native issue dependency를 우선 사용한다.
- native 기능을 사용할 수 없으면 본문에 `Blocked by: #<number>`를 기록한다.
- claim은 `gh issue edit <number> --add-assignee @me`로 처리한다.
- resolve는 답변 댓글을 작성하고 이슈를 닫은 뒤 map의 결정 사항을 갱신한다.
