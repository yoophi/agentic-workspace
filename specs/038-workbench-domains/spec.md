# Feature Specification: 나머지 도메인의 Workbench 이관 (서버-클라이언트 전환 1b)

**Feature Branch**: `038-workbench-domains`

**Created**: 2026-09-26

**Status**: Draft

**Input**: User description: "다음 작업 진행" — 2026-09-26 grill 세션에서 확정한 시리즈 순서상 다음 단계인 **038(1b)**: 037이 프로젝트 도메인에 만든 `Workbench` Seam 뒤로 나머지 Tauri command를 도메인 단위로 이관한다. 정본은 [서버-클라이언트 전환 조사](../../docs/client-server-architecture-research.md) 1단계, 절차 템플릿은 [Workbench Seam](../../docs/workbench-seam.md)의 "038 이후 이관 절차".

## 배경과 목적

037은 71개 Tauri command 중 2개(`list_projects`·`create_project`)만 새 인터페이스(`Workbench.call`)로 통과시켜 인터페이스·오류·멱등성·동시성·계약 생성 규칙이 성립함을 확인했다. 나머지 69개는 여전히 각 command가 저장소·Git·파일시스템을 직접 조립한다. 정본 1단계의 완료 조건은 "Tauri command가 repository/process를 직접 조립하지 않고 `Workbench`만 호출한다"이다.

이 spec은 그 조건을 **지금 옮길 수 있는 범위에서** 달성한다. 69개는 성격이 셋으로 갈린다.

| 구분 | 개수 | 이 spec에서의 처리 |
|---|---|---|
| **서버 소유 상태·조회** — 프로젝트 수정·삭제, saved prompt, goal, agent 실행 설정, Git·worktree·파일 조회, worktree 생성·삭제, agent catalog·provider 세션 조회 | 29 | **이관** (US1–US3) |
| **이벤트·창 정체에 묶인 장기 작업** — agent run 8, agent exchange 4, orchestration 18, worktree watcher 2 | 32 | 이관하지 않고 **목록과 이유를 기록** (US4). 이 command들은 창 label로 소유자를 정하고 Tauri 이벤트로 결과를 흘리므로, 정본 2단계(이벤트 봉투·`window_label` 분해)와 함께 옮겨야 한다 |
| **데스크톱 표현 상태** — 글꼴 크기 3, panel layout 2, 창 열기 2, 외부 URL 1 | 8 | 이관 대상이 아님. 정본 배치표대로 데스크톱 셸에 영구히 남는다 |

목적은 세 가지다. (1) 037의 "도메인 이동 템플릿"이 저장 파일 4개·Git CLI·파일시스템·환경 설정처럼 성격이 다른 도메인에도 그대로 통하는지 확인한다. (2) 변경 기록(ledger) 규칙이 JSON이 아닌 **외부 부작용**(worktree 생성·삭제)에도 성립하는지 확인한다. (3) 2단계가 정확한 남은 목록으로 시작할 수 있게 71개 전부의 분류를 문서로 고정한다.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 프로젝트 수정·삭제, saved prompt, goal, agent 실행 설정이 그대로 동작하고 세 경로에서 같은 결과를 낸다 (Priority: P1)

AW 사용자는 프로젝트를 고치거나 지우고, 자주 쓰는 프롬프트를 저장·수정·삭제하고, 작업 디렉터리별 목표를 만들고 진행을 기록하고, agent 실행 설정을 저장하는 일을 오늘과 똑같이 한다. 화면·응답 속도·저장 파일 위치와 형식·오류 문구는 바뀌지 않는다. 같은 13개 기능이 데스크톱을 거치지 않는 메모리 내 경로와 테스트용 로컬 HTTP 경로에서도 같은 입력에 같은 결과와 같은 오류 코드를 낸다. 상태를 바꾸는 요청은 037의 프로젝트 생성과 같은 규칙(멱등성 키 필수, 의도 기록 → 적용 → 확정)으로 처리된다.

**Why this priority**: 037과 가장 닮은 도메인(저장 파일 기반)이어서 템플릿 검증 효과가 가장 크고, 037이 절반만 옮긴 프로젝트 수정·삭제를 마무리한다. 사용자가 매일 쓰는 기능이므로 회귀 여부도 가장 빨리 드러난다.

**Independent Test**: 기존 프로젝트·saved prompt·goal·설정 관련 자동 테스트가 수정 없이 통과하고, 13개 기능 각각의 성공·검증 실패·없는 대상 fixture를 세 경로에 보내 결과와 오류 코드를 비교한다. 변경 기능은 같은 키 재요청·다른 내용 재요청·처리 중 중단 fixture를 추가로 통과한다.

**Acceptance Scenarios**:

1. **Given** saved prompt가 N개 저장된 상태에서, **When** 사용자가 목록을 열고 하나를 수정한 뒤 다른 하나를 삭제하면, **Then** 이전 버전과 같은 순서·내용으로 표시되고 저장 파일의 위치·형식이 이전 버전이 만든 것과 호환된다.
2. **Given** 작업 디렉터리 D에 목표가 없을 때, **When** 목표를 만들고 진행을 두 번 기록한 뒤 조회하면, **Then** 진행 기록 두 건이 순서대로 보이고, 같은 요청을 메모리 내 경로·HTTP 경로로 보내도 항목 단위로 일치한다.
3. **Given** 프로젝트 P가 있을 때, **When** 이름을 빈 값으로 수정하려 하면, **Then** 세 경로 모두 같은 안정적 오류 코드로 거절하고 데스크톱에는 이전과 같은 문구가 표시된다.
4. **Given** 존재하지 않는 프로젝트 ID로 삭제를 요청하면, **When** 세 경로 어디로 보내도, **Then** 같은 "없음" 오류 코드가 돌아오고 저장 파일은 변하지 않는다.
5. **Given** 목표 진행 기록 요청이 멱등성 키 K로 성공한 뒤, **When** 같은 키 K와 같은 내용으로 다시 보내면, **Then** 진행 기록이 하나만 남고 첫 응답과 같은 결과가 돌아온다.
6. **Given** agent 실행 설정 저장 중 저장 직후·확정 직전에 앱이 강제 종료됐을 때, **When** 재시작 뒤 같은 키로 재조회하면, **Then** 자동 재실행 없이 적용 여부가 `적용됨`·`적용 안 됨`·`불명` 중 하나로 정확히 판정된다.

---

### User Story 2 - Git·worktree·파일 조회와 worktree 생성·삭제가 서버 경계에서 같은 안전 규칙으로 동작한다 (Priority: P2)

AW 사용자는 원격·브랜치·worktree 목록을 보고, 변경 파일과 diff를 보고, 커밋 이력·그래프·커밋 상세를 훑고, worktree 안의 파일 목록과 텍스트 내용을 미리 보고, worktree를 만들고 지운다. 이 14개 기능은 앱 데이터가 아니라 **사용자의 Git 저장소와 파일시스템**을 다루며, 오늘 적용되는 안전 규칙(작업 디렉터리 밖 경로 거절, 미리보기 크기 상한, UTF-8 아닌 파일 처리, 빈 경로 거절)이 그대로 지켜진다. worktree 생성·삭제는 되돌리기 어려운 외부 부작용이므로 변경 기록 규칙을 따르되, 재시작 뒤 판정은 저장 파일이 아니라 **실제 Git 상태를 관찰해** 내린다.

**Why this priority**: 정본이 "첫 slice 뒤 read-only Git/file operation을 같은 방식으로 넓혀 Seam이 충분한지 조기에 검증하라"고 권고한 바로 그 지점이다. 네이티브 프로세스(Git CLI)·파일시스템 실패를 안정적 오류 코드로 분류하는 첫 사례이자, 변경 기록이 JSON 밖 부작용에도 통하는지 보는 유일한 기회다. P1과 독립이지만 사용 빈도와 회귀 위험은 P1보다 낮다.

**Independent Test**: 임시 Git 저장소 fixture(커밋 3개, 브랜치 2개, 변경 파일 2개)에 대해 14개 기능의 성공 fixture와 실패 fixture(저장소 아님, 빈 경로, 디렉터리 밖 경로, 없는 커밋, 크기 상한 초과, UTF-8 아님)를 세 경로에 보내 결과·오류 코드를 비교한다. worktree 생성은 같은 키 재요청과 처리 중 중단 뒤 재시작 판정을 추가로 통과한다.

**Acceptance Scenarios**:

1. **Given** 커밋 3개가 있는 저장소에서, **When** 사용자가 커밋 이력·그래프·첫 커밋 상세·그 커밋의 파일 diff를 순서대로 열면, **Then** 이전 버전과 같은 내용이 표시되고 세 경로의 응답이 항목 단위로 일치한다.
2. **Given** worktree 안에 512KB를 넘는 텍스트 파일과 UTF-8이 아닌 파일이 있을 때, **When** 각각을 미리 보면, **Then** 이전 버전과 같은 방식으로 잘림 표시·읽기 불가 안내가 나오고 새 실패 모드가 생기지 않는다.
3. **Given** 작업 디렉터리 밖을 가리키는 경로(`../outside`)로 파일 읽기를 요청하면, **When** 세 경로 어디로 보내도, **Then** 같은 안정적 오류 코드로 거절되고 어떤 파일도 읽히지 않는다.
4. **Given** Git 저장소가 아닌 디렉터리로 브랜치 목록을 요청하면, **When** 세 경로 어디로 보내도, **Then** 같은 오류 코드와 이전과 같은 문구가 돌아온다.
5. **Given** worktree 생성 요청이 멱등성 키 K로 성공한 뒤, **When** 같은 키 K와 같은 내용으로 다시 보내면, **Then** worktree가 하나만 존재하고 첫 응답과 같은 결과가 돌아온다.
6. **Given** worktree 생성이 접수된 뒤 Git 명령이 끝나고 확정 전에 앱이 강제 종료됐을 때, **When** 재시작 뒤 같은 키로 재조회하면, **Then** 시스템이 실제 worktree 목록을 확인해 `적용됨`으로 판정하고, Git 명령 전에 종료됐다면 `불명`으로 판정하며 자동으로 다시 만들지 않는다.

---

### User Story 3 - agent catalog와 provider 세션 조회가 같은 인터페이스에서 제공된다 (Priority: P3)

AW 사용자는 실행할 수 있는 agent 목록을 보고, 선택한 provider가 로컬에 남긴 과거 세션을 작업 디렉터리별로 조회해 이어 붙인다. 두 기능은 앱 데이터도 Git도 아닌 **실행 환경**(환경 설정으로 정해지는 catalog, provider가 쓴 로컬 파일)을 읽는다. 결과는 오늘과 같고, 데스크톱 밖 경로에서도 같은 목록을 얻는다.

**Why this priority**: 작고 독립적이며 사용자 영향이 작다. 그러나 "앱 데이터를 건드리지 않는 서버 소유 조회"의 자리를 인터페이스에 마련해야 2단계의 run 시작(agent 선택·세션 재개)이 같은 catalog를 참조할 수 있다.

**Independent Test**: 환경 설정으로 agent 2개를 등록한 fixture와 provider 세션 파일 3개(작업 디렉터리 2종)를 둔 fixture에 대해, 목록·디렉터리별 필터·전체 조회·없는 agent 요청을 세 경로에 보내 결과를 비교한다.

**Acceptance Scenarios**:

1. **Given** agent 2개가 등록된 환경에서, **When** 사용자가 agent 선택 목록을 열면, **Then** 이전 버전과 같은 2개가 같은 순서로 보이고 세 경로의 응답이 일치한다.
2. **Given** provider X의 세션이 디렉터리 A에 2개, B에 1개 있을 때, **When** A로 필터해 조회하면 2개, 필터 없이 조회하면 3개가 **Then** 최신순으로 돌아오고 상한(50개)이 유지된다.
3. **Given** 등록되지 않은 provider ID로 세션을 조회하면, **When** 세 경로 어디로 보내도, **Then** 같은 오류 코드가 돌아온다.

---

### User Story 4 - 다음 단계 담당자가 71개 command 전부의 이관 상태와 이유를 한 곳에서 확인한다 (Priority: P4)

2단계(이벤트 통합)를 맡는 사람은 "어떤 command가 이미 새 인터페이스 뒤로 옮겨졌고, 어떤 것이 왜 남았으며, 어떤 것은 영구히 데스크톱에 남는지"를 코드를 뒤지지 않고 문서 한 곳에서 확인한다. 옮겨진 도메인의 원본 코드는 데스크톱 앱에서 사라져 두 벌이 공존하지 않는다. 계약 조회(`system.describe`)와 생성된 클라이언트 타입은 옮겨진 operation 전부를 담는다.

**Why this priority**: 정본이 지적한 "OpenWiki는 ~40개, 실제 71개" 불일치를 baseline inventory로 끝내고, 2단계의 범위 논쟁을 없앤다. 기능이 아니라 문서·정리 산출물이므로 마지막이다.

**Independent Test**: 문서의 분류표 행 수가 71이고 각 행이 세 구분 중 하나와 이유를 가지며, 데스크톱 앱 백엔드에 옮긴 도메인의 모델·규칙·저장 어댑터 파일이 남아 있지 않고, 계약 조회 결과의 operation 수가 문서의 "이관" 행 수 + 계약 조회 자체와 일치한다.

**Acceptance Scenarios**:

1. **Given** 이관이 끝난 뒤, **When** 담당자가 Seam 문서의 inventory 표를 보면, **Then** 71개 command가 `이관됨(037)`·`이관됨(038)`·`2단계로 이연`·`데스크톱 유지` 중 하나로 분류되어 있고 이연·유지 행에는 이유가 있다.
2. **Given** 개발자가 데스크톱 계약 조회를 호출하면, **When** 응답을 보면, **Then** 037의 3개와 038의 29개를 합친 32개 operation이 종류·입력·출력 계약·멱등성 요구와 함께 나오고, 조회 전용 호출자에게는 변경 operation 13개(038의 12개 + `project.create`)가 빠져 19개만 보인다.
3. **Given** 생성된 클라이언트 타입을 쓰는 코드가 있을 때, **When** 32개 operation 중 어느 것의 출력을 다른 operation의 출력 타입으로 다루면, **Then** 실행 전(컴파일) 단계에서 오류가 난다.

---

### Edge Cases

- **저장 파일(saved prompt·goal·설정) 손상**: 조회는 파일에 쓰지 않고 실패만 알린다. 백업으로부터의 복구는 그 저장 단위의 lock 안에서만 일어나므로, 손상을 발견한 조회가 동시에 진행 중인 변경을 덮어쓰지 못한다(037의 프로젝트 규칙을 옮기는 모든 저장 단위로 확장).
- **같은 저장 단위에 대한 동시 변경**: 예컨대 두 호출자가 같은 작업 디렉터리의 목표 진행을 동시에 기록하면 한 번에 하나만 적용되고 revision이 단조 증가하며 어느 기록도 사라지지 않는다. 호출자가 기대 revision을 보냈고 어긋나면 거절한다.
- **변경 기록 저장소를 열 수 없는 경우**: 변경은 일시 불가로 거절되고 조회는 계속된다. 저장이 **이미 끝난 뒤** 확정만 실패하면 `불명`으로 알리고 진행 기록을 남긴다.
- **worktree 생성이 절반만 된 경우**(디렉터리는 생겼지만 Git 등록이 안 됨): 재시작 판정은 Git이 인식하는 worktree 목록을 기준으로 하며, 인식되지 않으면 `불명`이다. 시스템이 임의로 지우거나 다시 만들지 않는다.
- **worktree 삭제 대상이 이미 없는 경우**: 같은 키 재요청은 첫 결과를 재생하고, 새 키 요청은 "없음" 오류 코드를 낸다.
- **Git 실행 파일이 없거나 명령이 실패한 경우**: 사용자 입력 문제(저장소 아님, 없는 커밋·브랜치)와 환경 문제(Git 없음, 권한 없음)를 다른 오류 코드로 구분한다. 문구는 오늘 표시되는 것을 유지한다.
- **작업 디렉터리 밖 경로, 빈 경로, 심볼릭 링크로 탈출하는 경로**: 실제 경로를 확인해 디렉터리 안인지 판정하고 밖이면 거절한다. 오늘의 규칙을 서버 경계로 옮기되 약화하지 않는다.
- **미리보기 크기 상한 초과·UTF-8 아닌 파일**: 오늘과 같이 잘림 표시 또는 읽기 불가로 처리한다. 상한값은 바꾸지 않는다.
- **provider 세션 파일이 손상되었거나 형식이 다른 경우**: 그 항목만 건너뛰고 나머지를 돌려주는 오늘의 동작을 유지한다.
- **이관되지 않은 command**(run·exchange·orchestration·watcher·표현 상태): 오늘과 완전히 같게 동작하고, 계약 조회 목록에 나타나지 않는다.
- **데스크톱에서 같은 버튼을 두 번 빠르게 누른 경우**: 데스크톱 어댑터는 호출마다 새 멱등성 키를 만든다. 따라서 오늘처럼 두 번 실행되며, 재시도 중복 제거는 네트워크 전환(4단계)에서 클라이언트가 키를 재사용할 때 효력을 낸다. 이 spec은 UI 동작을 바꾸지 않는다.
- **변경 기록 보존 기간(24h)이 지난 뒤 재시도**: 새 요청으로 처리된다. 진행 중 기록은 만료되지 않는다.
- **자주 발생하는 작은 변경**(목표 진행 기록): 변경 기록을 거치므로 지연이 늘 수 있다. 사용자가 느끼는 지연 증가는 50ms 이내여야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 아래 표의 29개 기능을 037과 같은 **하나의 호출 인터페이스**(operation 이름 + 구조화된 입력 → 결과 또는 오류)로 제공해야 한다. 이벤트·창 정체에 묶인 32개 command(run 8·exchange 4·orchestration 18·watcher 2)는 이 spec에서 옮기지 않고 정본 2단계로 이연한다(2026-09-26 grill Q1 확정).

  | 현재 command | operation | 종류 | 저장 단위 / 부작용 |
  |---|---|---|---|
  | `update_project` | `project.update` | 변경 | projects |
  | `delete_project` | `project.delete` | 변경 | projects |
  | `list_saved_prompts` | `savedPrompt.list` | 조회 | saved-prompts |
  | `create_saved_prompt` | `savedPrompt.create` | 변경 | saved-prompts |
  | `update_saved_prompt` | `savedPrompt.update` | 변경 | saved-prompts |
  | `delete_saved_prompt` | `savedPrompt.delete` | 변경 | saved-prompts |
  | `get_goal` | `goal.get` | 조회 | goals |
  | `create_goal` | `goal.create` | 변경 | goals |
  | `update_goal` | `goal.update` | 변경 | goals |
  | `clear_goal` | `goal.clear` | 변경 | goals |
  | `record_goal_progress` | `goal.recordProgress` | 변경 | goals |
  | `get_agent_run_settings` | `agentRunSettings.get` | 조회 | agent-run-settings |
  | `save_agent_run_settings` | `agentRunSettings.save` | 변경 | agent-run-settings |
  | `list_git_remotes` | `git.listRemotes` | 조회 | 사용자 저장소(읽기) |
  | `list_git_branches` | `git.listBranches` | 조회 | 사용자 저장소(읽기) |
  | `list_git_worktrees` | `git.listWorktrees` | 조회 | 사용자 저장소(읽기) |
  | `create_git_worktree` | `git.createWorktree` | 변경 | 사용자 저장소(외부 부작용) |
  | `delete_git_worktree` | `git.deleteWorktree` | 변경 | 사용자 저장소(외부 부작용) |
  | `list_worktree_changes` | `worktree.listChanges` | 조회 | 사용자 저장소(읽기) |
  | `get_worktree_changes` | `worktree.getChanges` | 조회 | 사용자 저장소(읽기) |
  | `get_worktree_file_diff` | `worktree.getFileDiff` | 조회 | 사용자 저장소(읽기) |
  | `list_worktree_files` | `worktree.listFiles` | 조회 | 사용자 파일시스템(읽기) |
  | `read_worktree_text_file` | `worktree.readTextFile` | 조회 | 사용자 파일시스템(읽기) |
  | `list_worktree_git_history` | `worktree.listHistory` | 조회 | 사용자 저장소(읽기) |
  | `get_worktree_git_graph` | `worktree.getGraph` | 조회 | 사용자 저장소(읽기) |
  | `get_worktree_commit_detail` | `worktree.getCommitDetail` | 조회 | 사용자 저장소(읽기) |
  | `get_worktree_commit_file_diff` | `worktree.getCommitFileDiff` | 조회 | 사용자 저장소(읽기) |
  | `list_agents` | `agent.list` | 조회 | 실행 환경(읽기) |
  | `list_provider_sessions` | `agent.listProviderSessions` | 조회 | provider 로컬 파일(읽기) |

  표의 operation 이름은 확정된 계약 이름이다(2026-09-26 grill Q2·Q7): `git.*`는 저장소 단위, `worktree.*`는 체크아웃 디렉터리 단위, 나머지 도메인은 모델 이름의 camelCase를 쓴다. command ↔ operation 대응과 종류는 범위의 정의다.

- **FR-002**: 위 29개 기능의 데스크톱 command는 이 인터페이스만을 통해 동작해야 하며, 사용자에게 보이는 동작·오류 문구·응답 형태·저장 파일 위치·저장 형식은 변하지 않아야 한다. 데스크톱 command는 입력 변환과 결과 변환만 하고 저장소·Git·파일시스템을 직접 조립하지 않아야 한다.
- **FR-003**: 데스크톱 경로, 메모리 내 경로, 테스트용 로컬 HTTP 경로 세 호출 경로는 29개 operation 각각에 대해 같은 입력에 같은 결과와 같은 오류 코드를 반환해야 한다. operation마다 성공 1건 이상과 실패 1건 이상의 공통 fixture가 있어야 한다.
- **FR-004**: 변경 operation 12개는 037의 규칙을 그대로 따라야 한다: 멱등성 키 필수, 부작용 전 `대기` 기록 → 저장 단위 lock 안에서 적용과 revision 증가 → `적용됨` 확정 → 응답. 같은 키·같은 내용은 결과 재생, 같은 키·다른 내용은 충돌, 진행 중 기록은 `불명` 충돌. 재시작 시 미확정 기록은 자동 재실행하지 않고 판정한다.
- **FR-005**: 외부 부작용을 내는 변경(worktree 생성·삭제)의 재시작 판정은 **종료 상태 규칙**을 따라야 한다(2026-09-26 grill Q4 확정). `대기` 기록에는 서버가 기본값을 채운 뒤의 최종 worktree 경로가 남아야 하며, 재시작 시 생성은 그 경로가 저장소의 worktree 목록에 **있으면** `적용됨`, 삭제는 **없으면** `적용됨`, 그 외는 `불명`으로 판정한다. 시스템은 부분 상태를 임의로 정리하거나 재실행하지 않아야 한다. 이 규칙은 037의 "예약 id 증거" 규칙과 다르며, 원하는 종료 상태가 성립하면 누가 만들었는지 구별하지 않는다.
- **FR-006**: 조회 operation은 저장 파일에 쓰지 않아야 한다. 손상 저장 파일의 백업 복구는 그 저장 단위의 lock 안에서만, 저장 문서와 같은 타입으로 검증한 뒤 수행해야 한다. 옮겨진 저장 단위 4개 모두에 적용되며, 옮긴 뒤 데스크톱 앱에 lock 없는 복구 경로가 남아 있지 않아야 한다.
- **FR-007**: 사용자 저장소·파일시스템을 읽는 operation은 오늘의 안전 규칙(작업 디렉터리 실제 경로 밖 거절, 빈 경로 거절, 미리보기 크기 상한, UTF-8 아닌 내용 처리, 숨김·제외 디렉터리 규칙)을 서버 경계에서 동일하게 적용해야 한다. 규칙의 값과 결과는 바뀌지 않아야 한다.
- **FR-008**: 오류는 정본의 안정적 오류 코드 표로 분류되어야 한다. 저장 단위 변경은 037과 같이 입력 검증 실패·대상 없음·권한 없음·충돌·stale revision·일시 불가·내부 오류를 구분한다. Git·파일시스템 operation은 **Git이 낸 오류 문장을 해석하지 않고** 사전 검증만 분류한다(2026-09-26 grill Q5 확정): 입력 검증 실패 → 입력 오류, 작업 디렉터리 밖 경로 → 권한 없음, 사전에 확인 가능한 디렉터리·파일·저장소 없음 → 대상 없음, Git 실행 파일을 찾을 수 없음 → 일시 불가, Git 명령 비정상 종료 → 내부 오류(재시도 불가). 오류 message는 사람이 읽는 한 문장이며 데스크톱에 표시되는 기존 문구(Git 비정상 종료는 Git이 낸 문장 그대로)를 유지해야 한다.
- **FR-009**: 계약 조회 operation은 호출자에게 허용된 operation 전부(037의 3개 + 이 spec의 29개)를 종류·입력·출력 계약·멱등성 요구와 함께 반환해야 한다. 권한 범위는 도메인별 조회/변경으로 나뉘어야 하며, 데스크톱 호출자는 전부, 테스트용 조회 전용 호출자는 조회만 허용된다. 허용되지 않은 operation은 목록에 없고 호출도 거절된다.
- **FR-010**: 계약 정의 한 곳에서 전송 계약 문서와 클라이언트 타입이 생성되어야 하고, 32개 operation 모두 입력·출력이 짝지어진 타입으로 표현되어 불일치가 실행 전 단계에서 검출되어야 한다. 정의와 생성물이 어긋나면 저장소 검증이 실패해야 한다.
- **FR-011**: 옮겨진 도메인의 모델·업무 규칙·포트·저장 어댑터·Git·파일 어댑터는 공유 crate로 이동하고 데스크톱 앱 백엔드에서 삭제되어야 한다. 같은 규칙이 두 곳에 공존하지 않아야 한다.
- **FR-012**: Seam 문서에 71개 command 전부의 분류표(`이관됨(037)`·`이관됨(038)`·`2단계로 이연`·`데스크톱 유지`)와 이연·유지 이유가 기록되어야 하며, 정본 문서의 진행 상태 각주가 갱신되어야 한다.
- **FR-013**: 이 spec은 데스크톱 화면, 프론트엔드 통신 방식, run·exchange·orchestration·watcher command, 데스크톱 표현 상태 command(글꼴·layout·창 열기·외부 URL), 이벤트 전달 방식, 운영 환경 HTTP 노출을 변경하지 않아야 한다.

### Key Entities *(include if feature involves data)*

- **Saved Prompt**: 사용자가 저장한 재사용 프롬프트. 제목·본문·식별자·생성/수정 시각. 저장 단위 `saved-prompts`.
- **Thread Goal**: 작업 디렉터리 하나에 붙는 목표와 그 진행 기록 목록. 작업 디렉터리로 조회·수정·삭제한다. 저장 단위 `goals`.
- **Agent Run Settings**: 작업 디렉터리별 agent 실행 설정(명령 프로필 재정의 포함). 저장 단위 `agent-run-settings`.
- **Project**(037에서 이동): 수정·삭제가 이번에 합류한다. 저장 단위 `projects`.
- **Git 조회 결과**: 원격, 브랜치, worktree(상태 포함 여부 선택), 변경 파일 목록, 그룹화된 변경 요약, 파일 diff, 커밋 이력(페이지 커서), 커밋 그래프, 커밋 상세, 커밋 파일 diff. 모두 사용자 저장소를 읽어 만든 값이며 앱이 저장하지 않는다.
- **Worktree 파일**: 범위(전체/워크스페이스) 필터가 있는 파일 목록과 크기 상한·UTF-8 규칙이 적용된 텍스트 미리보기.
- **Worktree 생성 요청**: 경로·브랜치·기준 참조(모두 선택, 기본값 규칙 존재). 결과는 외부 부작용이며 재시작 판정은 Git 상태 관찰로 한다.
- **Agent Descriptor / Provider Session**: 환경 설정이 정의하는 agent 목록과, provider가 로컬에 남긴 세션 메타데이터(작업 디렉터리·시각·식별자). 읽기 전용.
- **Operation Inventory**: 71개 command 각각의 분류·이유·대응 operation을 담는 표. 2단계의 입력.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: `crates/workbench-protocol`(operation·입출력 계약 추가), `crates/workbench-core`(도메인·규칙·포트·저장·Git·파일 어댑터 이동과 handler), `apps/agentic-workbench/src-tauri`(29개 command를 호환 어댑터로 교체, 옮긴 파일 삭제), `packages/workbench-client`(생성물 갱신), `docs/`. 이미 공유 crate인 `crates/git-core`·`crates/acp-agent-core`는 core가 의존만 한다. 다른 앱은 건드리지 않는다.
- **Frontend layering**: UI 변경 없음. `apps/agentic-workbench/src/**` diff 0건이 완료 조건이다.
- **Backend boundary**: domain(모델)·application(규칙·handler)·ports(저장소·Git·파일 provider 추상)는 core로, infrastructure(JSON·Git CLI·파일시스템 어댑터)는 core의 infrastructure로 이동한다. AW inbound는 입력 변환→`Workbench.call`→결과 변환만 남는다. 포트는 core의 기존 `ports` 모듈 한 곳에 둔다.
- **Shared core vs UI**: 순수 core만 공유한다. 공유 UI 없음.
- **Persistence and safety**: 저장 단위 4개는 lock·revision·intent-first ledger 뒤에 놓인다. 파일 읽기는 실제 경로 기준 root 검사·크기 상한·UTF-8 처리를 core 경계에서 수행한다. run/session/permission owner 범위는 이 spec의 대상이 아니다(2단계).
- **Documentation and Storybook**: `docs/workbench-seam.md`에 상태·inventory·실제 적용된 절차를 갱신하고 `docs/client-server-architecture-research.md` 진행 각주를 갱신한다. OpenWiki는 자동 재생성에 맡긴다. Storybook N/A.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 프로젝트·saved prompt·goal·설정·Git·worktree·파일·agent catalog·provider 세션에 관한 기존 자동 테스트가 수정 없이 100% 통과하고, 사용자가 29개 기능을 쓸 때 조회·변경 모두 이전 버전 대비 지연 증가가 50ms 이내다.
- **SC-002**: 29개 operation 각각의 공통 fixture(성공·실패, 변경 operation은 멱등 재생·다른 내용 충돌·stale revision 포함)에 대해 세 호출 경로의 결과와 오류 코드가 100% 일치한다.
- **SC-003**: 저장 단위 4개 각각의 변경 하나와 worktree 생성에 대해, 처리 중 세 지점(기록 직후·적용 직후·확정 직전)에서 중단·재시작하는 fixture 전부에서 적용 여부 판정이 계약과 100% 일치하고 중복 생성이 0건이다.
- **SC-004**: 같은 저장 단위에 동시에 20건의 변경을 보내면 정확히 20건이 적용되고 revision이 단조 증가하며 사라진 변경이 0건이다.
- **SC-005**: 디렉터리 밖 경로·빈 경로·크기 상한 초과·UTF-8 아님·저장소 아님·없는 커밋 fixture가 이전 버전과 같은 결과(거절 또는 잘림·불가 표시)를 100% 재현한다.
- **SC-006**: 데스크톱 호출자의 계약 조회 결과가 정확히 32개 operation을 담고, 조회 전용 호출자의 결과에는 변경 operation이 0개이며 호출 시 100% 거절된다. 생성물 drift 검증이 통과하고, 32개 operation 중 어느 것이든 출력 타입을 잘못 다루는 코드는 컴파일에 실패한다.
- **SC-007**: 데스크톱 앱 백엔드에서 옮긴 도메인의 모델·규칙·저장·Git·파일 어댑터 파일이 0개 남고, 29개 command 본문에 저장소·Git·파일시스템 직접 조립이 0건이다. 프론트엔드 diff는 0건이다.
- **SC-008**: Seam 문서의 inventory 표가 71개 command 전부를 분류하고, 이연 32개·유지 8개 행 100%에 이유가 있다.

## Assumptions

- **범위 기준선**: 이벤트·창 정체 32개는 이연으로 확정되었다(grill Q1, [`docs/adr/0001`](../../docs/adr/0001-defer-event-bound-commands-to-stage-2.md)). 이유는 그 32개가 `window` label로 소유자와 이벤트 대상을 정하고 Tauri 이벤트 sink·메모리 journal·MCP 서버 상태에 묶여 있어, 창 정체를 분해하지 않고 옮기면 계약에 데스크톱 정체를 임시로 심어야 하고 2단계에서 다시 뜯어내야 하기 때문이다.
- **데스크톱 표현 상태 8개는 "이연"이 아니라 "유지"**다: 글꼴 크기 3개, panel layout 2개, 창 열기 2개, 외부 URL 열기 1개. 정본 배치표가 이를 데스크톱 셸/클라이언트별 presentation 저장소로 분류했다.
- **orchestration의 조회 2개**(`list_recoverable_orchestration_workspaces`·`replay_orchestration_runtime_events`)는 창에 묶이지 않지만 도메인을 쪼개지 않기 위해 orchestration 전체와 함께 2단계로 이연한다.
- **저장 단위(aggregate)는 저장 파일 하나 = 하나**다(2026-09-26 grill Q3 확정): `projects`·`saved-prompts`·`goals`·`agent-run-settings`. 각각 독립 revision을 가진다. 서로 다른 worktree의 goal 변경이 같은 lock·revision을 공유하는 것은 감수한다(데스크톱은 기대 revision을 보내지 않고, 저장이 SQLite로 옮겨질 때 입도를 줄인다). Git·파일 조회는 저장 단위가 없고, worktree 생성·삭제는 같은 사용자 저장소 안에서 직렬화한다.
- **데스크톱 어댑터는 호출마다 새 멱등성 키를 만든다**(037과 같음). UI 동작(두 번 누르면 두 번 실행)은 바꾸지 않는다.
- **변경 기록 보존 기간 24h, 기록 파일 위치, principal 두 종류(데스크톱 전체·테스트 조회 전용)**는 037 기본값을 그대로 쓴다. 권한 범위 이름은 도메인별 `<도메인>:read`/`<도메인>:write`로 늘린다.
- **저장 파일 4개의 형식과 위치는 불변**이다(시리즈 결정 3). 새 필드를 넣지 않는다.
- **Git·파일 어댑터는 이미 Tauri에 의존하지 않으므로**(작업 디렉터리 문자열만 받음) 이동 비용이 낮다. 안전 규칙의 값(미리보기 512KB, 숨김·제외 디렉터리, 세션 조회 상한 50)은 바꾸지 않는다.
- **Git CLI·파일시스템·provider 세션 어댑터는 전부 `workbench-core`로 옮기고 공유 crate `git-core`는 건드리지 않는다**(2026-09-26 grill Q6 확정). AW 화면 형태에 맞춰진 모델(worktree 삭제 가능 여부, inline diff)이 두 앱이 쓰는 crate에 섞이지 않게 하고, git-explorer 재검증을 피한다.
- **agent catalog는 기존 공유 crate(`acp-agent-core`)의 환경 설정 기반 catalog를 그대로 읽는다.** 새 설정 형식을 만들지 않는다.
- **오류 문구는 현재 데스크톱에 표시되는 문자열을 그대로 보존**한다. 코드는 정본 표에서 고르고 문구는 바꾸지 않는다.
- **이관 순서는 P1 → P2 → P3 → P4**지만 하나의 PR로 main에 squash merge한다(시리즈 결정 2). 중간 상태에서도 앱은 항상 동작해야 한다.
- 다음 단계(039, 정본 2단계)는 이 spec의 inventory 표를 시작점으로 삼는다.
