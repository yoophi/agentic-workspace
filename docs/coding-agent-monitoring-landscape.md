# 코딩 에이전트 작업 상태 모니터링 도구 조사

조사일: 2026-08-12 16:16 KST

## 범위와 읽는 법

여기서 말하는 모니터링은 단순 토큰/비용 집계보다 넓다. 여러 코딩 에이전트의 **실행 상태, 현재 작업, 입력·권한 대기, 결과물과 다음 조치**를 한 화면에서 파악하게 하는 도구를 골랐다. 따라서 다음 두 부류를 함께 비교한다.

- **관찰 우선(attach)**: 사용자가 터미널에서 시작한 기존 세션이나 로컬 상태 파일을 읽는다. 대시보드를 닫아도 작업이 계속되고, 도구가 실행 주체가 아닐 수 있다.
- **운영 우선(orchestrate)**: 도구가 세션·worktree를 만들고 상태·검토·피드백 루프까지 관리한다. 병렬 실행을 안정적으로 운영하기 좋지만, 대개 해당 도구 안에서 작업을 시작해야 하는 비중이 크다.

스타 수는 GitHub REST API의 `stargazers_count`를 위 시각에 재조회한 값이며, 이후에도 변할 수 있다. 각 항목의 “소개”와 “해결하려는 문제”는 저장소 README 및 메타데이터(모두 1차 출처)를 압축한 것이다.

**지원 범위 표기법**: `Claude Code 전용`은 다른 coding-agent 엔진의 연결·실행·관찰을 README에서 명시하지 않은 경우다. `다중 엔진`은 Codex를 포함해 2개 이상을 명시 지원한다. `범용 CLI`는 임의 터미널 프로그램을 실행할 수 있다는 뜻이며, 모든 엔진에서 같은 수준의 세션 파싱·상태 판별·후속 제어를 보장한다는 뜻은 아니다. 상세 근거와 기능 제한은 [지원 범위 감사](./coding-agent-support-audit.md)를 참조한다.

## 한눈에 보기

| 프로젝트 | 유형 · 지원 엔진 | GitHub star | 간략한 소개 | 프로젝트가 해결하려는 문제 |
|---|---|---:|---|---|
| [CCC — Claude Command Center](https://github.com/amirfish1/claude-command-center) | 관찰+경량 운영 · **다중 8개**: Claude Code, Codex, Cursor, Antigravity, Kilo Code, Kimi Code, OpenCode, Devin | 116 | 머신의 세션을 한 로컬 보드에 모아 상태·필요한 사용자 조치·컨텍스트/비용 신호를 보여주고, 세션 생성·후속 지시도 제공한다. [README](https://github.com/amirfish1/claude-command-center#ccc) · [메타데이터](https://api.github.com/repos/amirfish1/claude-command-center) | 터미널에서 직접 시작하거나 재개한 작업은 일반 오케스트레이터에서 사라진다. 에이전트가 이미 기록한 로컬 상태·transcript를 원본으로 삼아, 수동 시작 세션까지 누락 없이 한곳에서 보고 병렬 작업을 이어가려 한다. |
| [Claude Task Viewer](https://github.com/L1AD/claude-task-viewer) | 관찰 · **Claude Code 전용** | 687 | Claude Code task를 실시간 Kanban, 활동 피드, 의존성 그래프, Gantt형 타임라인과 완료 알림으로 표시하는 로컬 웹 도구다. [README](https://github.com/L1AD/claude-task-viewer#claude-task-viewer) · [메타데이터](https://api.github.com/repos/L1AD/claude-task-viewer) | 복합 작업을 세부 task로 분해해도 상태·의존성이 터미널 안에만 남는다. 실제 task 파일을 감시해 현재 진행, 막힌 경로, 오래된 세션을 시각화한다. 작업 상태의 소유권은 Claude Code에 둔다. |
| [Agent Orchestrator](https://github.com/Untrivial-ai/agent-orchestrator) | 운영 우선 · **다중 26개 adapter**: Claude Code, Codex, Cursor, OpenCode, Aider 등 | 9,388 | 병렬 코딩 에이전트를 격리 worktree에서 실행하고, 세션·터미널·브랜치·PR 및 피드백 루프를 단일 Agentic IDE에서 감독한다. [README](https://github.com/Untrivial-ai/agent-orchestrator#what-is-agent-orchestrator) · [메타데이터](https://api.github.com/repos/Untrivial-ai/agent-orchestrator) | 에이전트를 병렬로 돌리면 브랜치/터미널을 잃어버리고 CI 실패, 리뷰 코멘트, merge conflict를 올바른 작업자에게 되돌리기 어렵다. 실행을 격리하고 상태를 통합해 자동 피드백 루프를 만들려 한다. |
| [Orca](https://github.com/stablyai/orca) | 운영 우선 · **범용 CLI** (Claude Code, Codex, OpenCode, Pi 등; 모든 CLI agent 실행) | 43,129 | 병렬 worktree의 Codex·Claude Code·OpenCode·Pi 등을 한곳에서 추적·조작하는 데스크톱/모바일 Agent Development Environment다. 동일 프롬프트의 fan-out, 알림, diff 주석·재지시, 원격 worktree를 제공한다. [README](https://github.com/stablyai/orca#features) · [메타데이터](https://api.github.com/repos/stablyai/orca) | 여러 에이전트와 worktree를 동시 실행할 때 터미널·결과·검토 흐름이 분산된다. 병렬 실험의 결과를 비교하고, 완료/주의 필요 시점을 놓치지 않으며, 검토 피드백을 바로 해당 에이전트에 돌려보내려 한다. |
| [Claude Squad](https://github.com/smtg-ai/claude-squad) | 운영 우선 · **다중**: Claude Code, Codex, Gemini CLI, Aider + 임의 로컬 프로그램 | 8,285 | `tmux`와 Git worktree를 결합한 TUI로 task별 에이전트 인스턴스를 만들고, 하나의 터미널에서 상태·diff를 보며 재개·적용·push한다. [README](https://github.com/smtg-ai/claude-squad#claude-squad) · [메타데이터](https://api.github.com/repos/smtg-ai/claude-squad) | 한 저장소에서 여러 작업을 병렬 처리할 때 파일 충돌과 인스턴스 관리가 어려워진다. task별 격리 workspace와 중앙 TUI로 동시에 실행하면서 변경을 검토한 뒤 적용하게 한다. |
| [Claude Code Agent Farm](https://github.com/Dicklesworthstone/claude_code_agent_farm) | 대규모 운영 · **Claude Code 전용** | 902 | 최대 50개까지의 Claude Code 세션을 위한 프레임워크다. 파일 lock, heartbeat/context 경고, tmux 모니터, 자동 재시작, HTML 실행 보고서를 제공한다. [README: 개요·기능](https://github.com/Dicklesworthstone/claude_code_agent_farm#-what-is-this) · [README: 모니터링](https://github.com/Dicklesworthstone/claude_code_agent_farm#-monitoring-dashboard) · [메타데이터](https://api.github.com/repos/Dicklesworthstone/claude_code_agent_farm) | 수십 개 에이전트를 같은 코드베이스에 투입하면 파일 충돌, 멈춤/idle 상태, 컨텍스트 소진과 진척 파악이 핵심 병목이 된다. lock 기반 조정과 health 모니터링·복구로 대규모 병렬 작업을 운영하려 한다. |
| [Claude Code Agent Monitor](https://github.com/hoangsonww/Claude-Code-Agent-Monitor) | 관찰 중심 · **다중 2개**: Claude Code, Codex | 900 | 네이티브 hook을 수집해 session·agent·tool·subagent 상태를 실시간 웹/데스크톱 대시보드, Kanban, 타임라인, 분석 화면으로 제공하는 self-hosted 도구다. [README: 개요](https://github.com/hoangsonww/Claude-Code-Agent-Monitor#overview) · [README: 동작 방식](https://github.com/hoangsonww/Claude-Code-Agent-Monitor#how-it-works) · [메타데이터](https://api.github.com/repos/hoangsonww/Claude-Code-Agent-Monitor) | 동시·장시간 실행은 터미널에서 불투명해 권한/입력 대기, 오류, subagent의 실제 활동을 늦게 알게 된다. hook 이벤트를 저장·방송해 현재 상태, 도구 사용과 병렬 관계를 관찰·분석하려 한다. |
| [abtop](https://github.com/graykode/abtop) | 관찰 · **다중 3개**: Claude Code, Codex CLI, OpenCode | 3,438 | `btop/htop` 같은 터미널 UI로 로컬 세션, token·컨텍스트 창·rate limit, child process, 열린 포트, Git 상태를 읽기 전용으로 보여준다. [README](https://github.com/graykode/abtop#abtop) · [메타데이터](https://api.github.com/repos/graykode/abtop) | 여러 프로젝트의 agent를 켜면 quota 소진, context 부족, 백그라운드 서버/고아 포트 같은 운영 문제를 탭을 오가며 찾아야 한다. API 키 없이 프로세스·파일 상태에서 이를 즉시 파악하게 한다. |
| [Open Island](https://github.com/Octane0411/open-vibe-island) | 관찰+상호작용 · **다중 10개**: Claude Code, Codex, Cursor, Gemini CLI, Kimi CLI, OpenCode, Qoder, Qwen Code, Factory, CodeBuddy | 1,915 | Mac notch/상단 바에 세션 상태와 권한 승인 요청을 띄우고, 적절한 터미널/IDE 세션으로 즉시 되돌아가게 하는 local-first 네이티브 앱이다. [README: 소개](https://github.com/Octane0411/open-vibe-island#what-is-open-island) · [README: 지원 범위](https://github.com/Octane0411/open-vibe-island#supported-agents--terminals) · [메타데이터](https://api.github.com/repos/Octane0411/open-vibe-island) | 개발 흐름을 깨지 않고도 agent가 끝났는지, 권한/답변을 기다리는지 알아야 한다. 별도 대시보드 탐색 대신 항상 보이는 작은 표면에서 알림·승인·정확한 세션 복귀를 제공한다. |
| [Claude Code Monitor](https://github.com/onikan27/claude-code-monitor) | 관찰+원격 제어 · **Claude Code 전용** | 294 | TUI와 휴대폰 웹 UI로 여러 Claude Code 세션을 실시간 확인하고, 지원 터미널에 포커스하거나 텍스트/권한 선택을 전달한다. [README](https://github.com/onikan27/claude-code-monitor#claude-code-monitor) · [메타데이터](https://api.github.com/repos/onikan27/claude-code-monitor) | 자리를 비웠거나 여러 터미널을 열었을 때 현재 세션과 permission prompt를 놓친다. hook·파일 기반 상태 및 로컬 네트워크/Tailscale 접근으로, 휴대폰에서도 상태 확인과 최소한의 개입을 가능하게 한다. |

## 문제 공간별 선택 가이드

| 필요 | 우선 살펴볼 도구 | 선택 이유 |
|---|---|---|
| 이미 수동으로 띄운 여러 세션을 빠짐없이 관찰 | CCC, abtop, Claude Code Monitor | 실행을 강제 소유하기보다 로컬 세션/프로세스/상태 파일을 수집하는 접근이다. |
| Claude Code가 만든 task의 상태·의존성만 명확히 보고 싶음 | Claude Task Viewer | task 파일을 source of truth로 두며 Kanban·의존성·타임라인이 가장 직접적이다. |
| 병렬 worktree, PR·CI·리뷰까지 운영 | Agent Orchestrator, Orca, Claude Squad | 세션 시작부터 격리와 결과 검토/피드백을 관리한다. GUI 폭은 Agent Orchestrator·Orca, 터미널 중심은 Claude Squad다. |
| 수십 개 Claude Code 작업자를 한 코드베이스에서 돌림 | Claude Code Agent Farm | file lock, heartbeat, 자동 복구처럼 고밀도 병렬 실행에 특화되어 있다. |
| 상태를 항상 눈에 두고 권한 요청에 바로 반응 | Open Island | macOS 상단 UI에 알림·승인·세션 복귀를 배치한다. |
| tool/subagent 이벤트와 실행 이력을 분석 | Claude Code Agent Monitor | hook 기반 event 저장과 Kanban/타임라인/분석을 함께 제공한다. |

## 공통적으로 해결하려는 핵심 문제

1. **가시성 상실**: 터미널 창과 worktree가 늘수록 무엇이 실행 중이고, 끝났고, 사람의 답을 기다리는지 알기 어렵다.
2. **병렬성의 부작용**: 동일 저장소에서 병렬 작업하면 파일·브랜치 충돌, 중복 작업, 누락된 결과 검토가 생긴다.
3. **handoff 지연**: CI 실패, 리뷰 코멘트, 권한 요청, context/rate-limit 경고가 해당 세션과 분리되어 대응이 늦어진다.
4. **사후 추적의 어려움**: 원시 transcript/상태 파일만으로는 어떤 도구를 썼고 어느 subagent가 막혔는지, 비용·시간이 어디에 들었는지 파악하기 힘들다.

이 차이는 제품 설계에서 중요하다. **관찰형은 실제 실행 상태와의 동기화·비침투성을**, **운영형은 격리·배분·검토 루프의 일관성을** 우선한다. 둘은 경쟁만 하는 관계가 아니라, 수동 세션이 많은 환경에서는 관찰형을 바깥 레이어로 두고 특정 병렬 작업에만 운영형을 쓰는 식으로 함께 쓸 수도 있다.

## 전체 생태계 지도와의 통합 관점

이 문서는 후보 선정용 압축본이다. Logseq의 [전체 조사](logseq://graph/private-zk?page=%EC%BD%94%EB%94%A9%20%EC%97%90%EC%9D%B4%EC%A0%84%ED%8A%B8%20%EC%9E%91%EC%97%85%20%EC%83%81%ED%83%9C%20%EB%AA%A8%EB%8B%88%ED%84%B0%EB%A7%81%20%EB%8F%84%EA%B5%AC%20%EC%A1%B0%EC%82%AC)는 웹·TUI·훅·원격·비용 도구까지 포괄한다. 둘을 합치면 제품 지형은 네 축으로 정리된다.

| 축 | 핵심 질문 | 대표 접근·도구 |
|---|---|---|
| 실제 상태 수집 | 이미 실행 중인 세션의 상태를 어떻게 정확하고 빠짐없이 읽는가? | task 파일·transcript·hook·프로세스 수집: Claude Task Viewer, CCC, abtop |
| 사람의 주의·승인 처리 | 누가 언제 사용자의 입력·권한 승인·검토를 기다리는가? | 우선순위·알림·세션 복귀: Open Island, Claude Code Monitor |
| 병렬 실행·검토 제어 | worktree·브랜치·CI·PR 피드백을 어느 작업자에게 연결하는가? | 격리·배분·검토 루프: Orca, Agent Orchestrator, Claude Squad, Agent Farm |
| 비용·사후 분석 | 비용·도구 호출·subagent 관계·실행 이력을 어떻게 진단하는가? | 이벤트 리플레이·token/rate-limit 분석: Claude Code Agent Monitor, claude-tap, ccusage |

따라서 이 문서의 10개 후보는 앞의 세 축을 실용적으로 비교하는 출발점이며, 전체 제품·아키텍처 판단에서는 네 번째 축과 이벤트 리플레이·원격 대응 후보도 함께 검토해야 한다.

## 출처와 재현 방법

- 스타 수: 각 행의 `메타데이터` GitHub REST API 응답의 `stargazers_count` (조회 시각: 2026-08-12 16:16 KST).
- 기능·문제 설명: 각 행에 연결한 해당 저장소의 README. 외부 블로그·비공식 비교 글은 근거로 사용하지 않았다.
- 재현 명령: `gh api repos/<owner>/<repo> --jq '.stargazers_count'`.

