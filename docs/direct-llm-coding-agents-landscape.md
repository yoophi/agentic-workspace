# LLM API를 직접 호출하는 코딩 에이전트 프로젝트 조사

조사일: 2026-08-12 17:09:28–17:09:36 KST
GitHub star: 해당 시각 `gh api repos/<owner>/<repo>`의 `stargazers_count`를 조회한 값이다. 별은 계속 변한다.

## 결론

**있다.** Claude Code나 Codex CLI를 다시 실행·모니터링하는 것이 아니라, 프로젝트 자신의 코드가 모델 공급자 API(또는 OpenAI-compatible/self-hosted endpoint)를 호출하고, 그 응답에 따라 파일 편집·셸 명령·테스트를 반복하는 에이전트들이다. 이 층은 모델 선택, 프롬프트/도구 loop, 권한, 실행 격리를 직접 설계할 수 있다는 점에서 앞서 조사한 작업 상태 모니터와 다르다.

다만 “직접 API”는 두 가지를 함께 포함한다.

- 사용자가 Anthropic·OpenAI·Google·OpenRouter 등의 API key/endpoint를 지정하는 전형적인 경로
- 일부 제품이 병행 제공하는 Claude/ChatGPT/Gemini 구독 로그인 또는 ACP 경로. 이것은 직접 API 경로와 구별해서 보아야 한다.

## 포함·제외 기준

포함하려면 (1) 저장소의 agent loop가 모델 completion/tool-call을 자체 호출하고, (2) 코딩 작업에서 파일 또는 명령 실행을 수행하며, (3) 공식 README·문서·소스에서 이를 확인할 수 있어야 한다. 따라서 Claude Code/Codex 같은 별도 실행파일을 단지 spawn하는 오케스트레이터·대시보드는 제외했다.

| 비교 | 직접 API 코딩 에이전트 | 기존 모니터링/CLI-wrapper 도구 |
|---|---|---|
| 모델 호출의 소유자 | 프로젝트 코드가 provider API/endpoint에 요청한다 | 대개 Claude Code·Codex CLI 등 이미 존재하는 세션이 요청한다 |
| 작업 loop | 자체 prompt → tool call → 파일/셸/테스트 → 재시도 | 세션 생성·전달·관찰·worktree/terminal 관리가 중심 |
| 변경 가능한 핵심 | 모델 라우팅, tool schema, 승인 정책, sandbox, retry | 세션 어댑터, 상태 수집, 알림, 병렬 실행/검토 UX |
| 대표 예 | Aider, Cline, OpenCode, mini-SWE-agent | CCC, Claude Task Viewer, abtop, Claude Squad |

두 접근은 대체재만은 아니다. 예를 들어 직접 API agent를 worktree/Docker에서 실행하고, 바깥의 관찰·오케스트레이션 도구로 여러 실행을 관리할 수 있다.

## 한눈에 보기

| 분류 | 프로젝트 | Star | 직접 모델 호출·공급자 | 작업 실행/격리 경계 | 해결하려는 문제 |
|---|---|---:|---|---|---|
| 자율 SWE | [mini-SWE-agent](#mini-swe-agent) | 6,401 | LiteLLM 직접 호출; 임의 LiteLLM 모델 | local Bash 또는 Docker/Podman 등 환경 | 최소한의 구조로 issue/버그 수정 agent를 실행·연구·평가 |
| 자율 SWE | [SWE-agent](#swe-agent-legacy) — legacy | 20,047 | LM 선택(GPT·Claude 등), LiteLLM 기반 | repo 작업을 Docker sandbox에서 수행 가능 | GitHub issue를 자율적으로 수정; 현재는 mini-SWE-agent 권장 |
| 비동기 SWE | [Open SWE](#open-swe) | 10,537 | LangGraph/Deep Agents 모델 설정(README 예: `openai:` 모델) | cloud sandbox, subagent, 자동 PR | 조직 내부의 Slack/Linear→비동기 수정→PR agent를 구성 |
| 대화형 | [Aider](#aider) | 48,137 | Anthropic/OpenAI/DeepSeek 및 local 등 API key | 현재 Git repo, diff·자동 commit으로 검토/되돌림 | 대화하면서 정확한 multi-file 변경을 만들고 Git으로 안전히 검토 |
| 대화형 | [Cline](#cline) | 66,032 | Anthropic, OpenAI, Google, OpenRouter, Ollama 등 | 파일/terminal·test; 승인 또는 auto-approve, Kanban은 worktree | IDE/CLI에서 계획→편집→명령/검증을 모델 선택권과 함께 수행 |
| 대화형 | [OpenCode](#opencode) | 196,366 | 모든 LLM provider 및 custom OpenAI-compatible provider | `build` full-access / `plan` read-only, 명령 권한 정책 | 특정 벤더 CLI에 묶이지 않고 로컬/원격 모델로 코딩 agent 수행 |
| 대화형·범용 | [Goose](#goose) | 52,704 | 15개 이상 provider: Anthropic, OpenAI, Google, Ollama 등 | 로컬 머신의 CLI/desktop/API와 확장 도구 | 코드 편집·명령·테스트를 포함한 확장 가능한 로컬 agent |
| 대화형·대형 작업 | [Plandex](#plandex) | 15,582 | Anthropic/OpenAI/Google/open source·OpenRouter 등의 key | cumulative diff review sandbox, 제어된 명령/rollback | 긴 컨텍스트·수십 파일·다단계 작업을 계획하고 안전하게 실행 |
| SDK | [OpenHands Software Agent SDK](#openhands-software-agent-sdk) | 981 | `LLM(model, api_key)` 및 LiteLLM 지원 provider | local workspace 또는 Docker/Kubernetes ephemeral workspace | 고정 제품 대신 production coding workflow의 agent loop·도구·격리를 조립 |
| 다중 에이전트 | [MetaGPT](#metagpt) | 69,783 | OpenAI/Azure/Ollama/Groq 등 설정 | 역할 기반 software-company workflow, Docker 설치 | 역할별 LLM 협업으로 요구→설계→코드 산출 흐름을 실험·구성 |
| 다중 에이전트 | [ChatDev](#chatdev) | 33,980 | provider API key/base URL 설정 | Docker Compose; 과거 v1은 Git mode | 여러 역할의 LLM workflow를 구성·실행. 코딩 특화 v1은 legacy branch |

## 프로젝트별 확인

### mini-SWE-agent

- **성격:** 경량 자율 소프트웨어 엔지니어링 agent. GitHub issue 또는 명령행 task를 한 개의 Bash 도구와 선형 history로 푼다.
- **직접 호출 근거:** 모델 어댑터가 `litellm.completion(model, messages, tools=[BASH_TOOL])`를 호출한다. 따라서 Claude/Codex CLI subprocess가 아니라 LiteLLM을 통한 provider 모델 호출이다. [모델 소스](https://github.com/SWE-agent/mini-swe-agent/blob/main/src/minisweagent/models/litellm_model.py#L1-L164) · [README](https://github.com/SWE-agent/mini-swe-agent)
- **실행 경계:** local environment는 Bash subprocess를, 별도 환경 구현은 Docker/Podman 등을 지원한다. 환경을 바꾸어 host 접근 범위를 통제할 수 있다. [local 환경 소스](https://github.com/SWE-agent/mini-swe-agent/blob/main/src/minisweagent/environments/local.py)
- **의미:** 대규모 제품의 UX보다 “작고 재현 가능하게 바꿀 수 있는 agent loop”를 원하는 경우의 출발점이다.

### SWE-agent (legacy)

- **성격:** GitHub issue를 받아 실제 repository에서 수정하도록 만든 연구 중심 자율 SWE agent다.
- **직접 호출 근거:** README는 GPT-4o·Claude Sonnet 등을 포함한 사용자의 언어 모델이 도구를 자율 사용한다고 설명하며, 구현은 LiteLLM completion과 API key 모델 설정을 둔다. [README](https://github.com/SWE-agent/SWE-agent#readme) · [모델 소스](https://github.com/SWE-agent/SWE-agent/blob/main/sweagent/agent/models.py#L15-L96)
- **실행 경계:** 공식 hello-world는 issue를 CLI로 고치고, Docker 기반 sandbox 실행 구성을 문서화한다. [튜토리얼](https://swe-agent.com/latest/usage/hello_world/)
- **주의:** README가 새 사용자는 **mini-SWE-agent**를 쓰라고 명시한다. 이 항목은 benchmark·학술적 계보를 위해 남겼으며, 새 구축 후보로는 위 mini를 우선 검토하는 편이 낫다.

### Open SWE

- **성격:** 조직 내부 코딩 agent를 만들기 위한 비동기 프레임워크다. Slack/Linear에서 호출하고 subagent가 cloud sandbox에서 작업한 뒤 PR을 만드는 패턴을 제공한다.
- **직접 호출 근거:** README가 LangGraph와 Deep Agents 위의 자체 harness를 설명하고 `create_deep_agent(model="openai:…", backend=sandbox_backend, ...)` 예제를 제공한다. 즉 외부 coding CLI를 감싸는 대신 모델·도구·middleware를 agent 코드에 조합한다. [README](https://github.com/langchain-ai/open-swe#readme)
- **실행 경계:** cloud sandbox와 자동 PR이 핵심 구성이다. worktree 자체를 주된 격리 단위로 내세우지는 않는다.
- **의미:** 개발자 개인의 대화형 CLI보다, 안전 경계·권한·사내 시스템 연결을 포함한 “조직 전용 coding worker”에 가깝다.

### Aider

- **성격:** 터미널 대화형 pair-programming agent.
- **직접 호출 근거:** README는 `--model` 및 `anthropic=<key>`, `openai=<key>` 예시를 들며 cloud·local LLM 연결을 명시한다. [README: cloud/local LLM](https://github.com/Aider-AI/aider#cloud-and-local-llms)
- **실행 경계:** 프로젝트의 Git과 밀결합하여 변경 diff를 만들고 자동 commit, lint/test 반복을 지원한다. 기본 격리는 별도 container/worktree가 아니라 Git diff·commit이다. [Git integration 문서](https://aider.chat/docs/git.html)
- **의미:** 모델을 직접 바꿔가며 개발자가 계속 검토하는 작은~중간 변경에 잘 맞는다.

### Cline

- **성격:** IDE extension·CLI·SDK를 공유 agent core 위에 둔 코딩 agent.
- **직접 호출 근거:** 공식 README는 Anthropic, OpenAI, Google, OpenRouter, Azure/Vertex, Ollama/LM Studio와 OpenAI-compatible API를 나열한다. SDK 문서는 `providerId: 'anthropic'`와 API key 설정도 보인다. [README: models](https://github.com/cline/cline#works-with-every-model) · [SDK provider 문서](https://docs.cline.bot/sdk/model-providers)
- **실행 경계:** agent는 파일을 편집하고 terminal에서 build/test를 실행한다. Plan/Act와 file/command별 승인, auto-approve가 있으며, 병렬 Kanban 작업은 worktree·auto-commit·dependency chain을 사용한다. [README: 실행·승인](https://github.com/cline/cline#edits-code-across-your-project)
- **주의:** Claude/Codex 구독/연결 경로도 공존할 수 있다. 여기서 말하는 직접 API 사용은 위 API-key provider를 고른 경로다.

### OpenCode

- **성격:** 터미널·desktop·web에서 쓰는 오픈소스 coding agent.
- **직접 호출 근거:** 공식 provider 문서는 “any LLM provider”를 내세우고, custom OpenAI-compatible provider와 provider별 인증을 구성한다. [Providers 문서](https://opencode.ai/docs/providers/) · [README](https://github.com/anomalyco/opencode#readme)
- **실행 경계:** `build`는 개발 작업 full access, `plan`은 read-only이며 Bash 실행 전에 permission을 묻는다. 설정의 permission rule과 `--auto`도 문서화되어 있다. [Agents](https://github.com/anomalyco/opencode#agents) · [Permissions](https://opencode.ai/docs/permissions/)
- **의미:** 모델·endpoint를 직접 바꿀 자유와 일상적인 agent 작업 UX를 함께 원하는 경우다. README/공식 문서에서 Docker/worktree 격리를 주력 기능으로 확인하지는 못했으므로, 기본적으로 현재 workspace의 권한 정책을 안전 경계로 간주해야 한다.

### Goose

- **성격:** code뿐 아니라 자동화·분석에도 쓰는 native desktop/CLI/API agent. 코딩 작업에는 edit·execute·test를 수행할 수 있다.
- **직접 호출 근거:** README가 Anthropic, OpenAI, Google, Ollama, OpenRouter, Azure, Bedrock 등을 포함한 15개 이상 provider를 API key로 사용한다고 명시한다. [README](https://github.com/aaif-goose/goose#readme)
- **실행 경계:** 로컬 머신에서 실행되며 MCP 확장으로 도구 범위를 넓힌다. 따라서 code agent로 쓸 때는 연결한 extension과 shell 권한이 실제 보안 경계다. README만으로는 작업별 Docker/worktree 격리가 핵심 기능임을 확인하지 못했다.
- **의미:** 하나의 provider/코드 편집 제품보다, 직접 호출 agent를 사내 도구·MCP와 결합하려는 경우에 맞는다.

### Plandex

- **성격:** 큰 코드베이스·긴 task에 초점을 둔 터미널 기반 계획·실행 agent.
- **직접 호출 근거:** Anthropic, OpenAI, Google, open-source provider 모델을 조합하며, self-host/local mode에서 OpenRouter 또는 다른 provider API key를 사용한다고 설명한다. [README: 모델·key](https://github.com/plandex-ai/plandex#provider-keys)
- **실행 경계:** AI 변경을 실제 프로젝트와 분리하는 **cumulative diff review sandbox**, 제어된 명령 실행과 rollback, 변경 이력 branch를 제공한다. Docker self-hosting도 가능하다. [README: 개요·sandbox](https://github.com/plandex-ai/plandex#what-is-plandex)
- **의미:** 수십 파일과 여러 단계에 걸친 기능에서 맥락 관리와 변경 누적 검토를 우선하는 선택지다.

### OpenHands Software Agent SDK

- **성격:** 완성된 단일 CLI라기보다, 코딩 task를 수행하는 agent를 직접 조립하는 SDK다.
- **직접 호출 근거:** quickstart가 `LLM(model='gpt-5.5', api_key=os.getenv('LLM_API_KEY'))`와 `TerminalTool`, `FileEditorTool`을 포함한 `Agent`를 구성하며, LiteLLM 지원 provider endpoint/key를 사용한다. [README quickstart](https://github.com/OpenHands/software-agent-sdk#quick-start) · [LLM settings](https://docs.openhands.dev/openhands/usage/settings/llm-settings)
- **실행 경계:** local workspace와 Docker/Kubernetes ephemeral workspace에서 terminal·file·MCP 도구를 실행하도록 구성할 수 있다. [SDK 문서](https://docs.openhands.dev/sdk)
- **중요한 구분:** 널리 알려진 [OpenHands/OpenHands](https://github.com/OpenHands/OpenHands)는 현재 Agent Canvas—OpenHands·Claude Code·Codex·Gemini/ACP agent를 실행·관리하는 control center—로 설명된다. 그것을 이 문서의 “직접 API agent” 사례로 세지 않았고, **Software Agent SDK**만 별도 직접 호출·agent construction 사례로 기록했다.

### MetaGPT

- **성격:** PM·architect·engineer 같은 역할을 LLM에 배분해 ‘software company’ workflow를 만드는 multi-agent framework다.
- **직접 호출 근거:** `config2.yaml` 예시가 `api_type: openai`와 Azure/Ollama/Groq 등의 provider, `base_url`과 모델명을 직접 설정한다. [README: 설정](https://github.com/FoundationAgents/MetaGPT#configuration)
- **실행 경계:** Docker 설치 경로가 있고 role/SOP 중심 orchestration을 제공한다. 특정 issue를 isolation된 worktree에서 고치는 제품이라기보다 산출물 생성·협업 구조를 연구/구성하는 프레임워크다.
- **의미:** 여러 독립 task의 병렬 실행보다 역할 기반 설계·구현 분업 자체를 실험하고 싶을 때 참고할 프로젝트다.

### ChatDev

- **성격:** 현재 v2(DevAll)는 설정으로 multi-agent workflow를 만드는 범용 플랫폼이다. 코딩 특화 ‘virtual software company’는 `chatdev1.0` legacy branch에 남아 있다.
- **직접 호출 근거:** README가 `.env`의 `API_KEY`/`BASE_URL`로 LLM provider를 설정하며 Python SDK에서 workflow를 실행하는 방법을 제공한다. v1 README는 `OPENAI_API_KEY` 설정을 명시한다. [현재 README](https://github.com/OpenBMB/ChatDev#readme) · [v1 README](https://github.com/OpenBMB/ChatDev/tree/chatdev1.0)
- **실행 경계:** Docker Compose와, v1의 Git mode가 문서화되어 있다. 그러나 현 main은 코드 수정 issue agent라기보다 다양한 workflow 실행 플랫폼이므로 일상 코딩 agent 후보로는 우선순위가 낮다.
- **의미:** 코드 생성 agent만을 찾는다면 앞선 도구가 더 직접적이고, ChatDev는 multi-agent 역할/대화 패턴의 역사적·실험적 비교군이다.

## 선택을 위한 요약

| 목표 | 우선 후보 | 이유 |
|---|---|---|
| issue를 재현 가능한 sandbox에서 자율 해결·평가 | mini-SWE-agent | 작은 agent loop, LiteLLM, 환경 교체가 명확하다. |
| 사내 task→sandbox→PR worker를 구축 | Open SWE, OpenHands SDK | 작업 수명주기와 backend/tool/permission을 코드로 조립한다. |
| 개인이 현재 repo를 대화식으로 수정 | Aider, Cline, OpenCode | 다중 provider 직접 호출, 파일·shell·test loop, 사용자 검토 UX가 있다. |
| 긴 task와 많은 파일을 안전히 누적 검토 | Plandex | diff sandbox와 rollback을 전면에 둔다. |
| 범용 자동화까지 같은 runtime으로 확장 | Goose | provider 폭과 MCP 확장성이 크다. |
| 역할 기반 다중 agent 연구/프로토타이핑 | MetaGPT, ChatDev | coding 작업 그 자체보다 orchestration 패턴을 중심으로 한다. |

## 설계상 시사점

직접 LLM API를 선택하면 모델 변경과 agent 정책을 통제할 수 있지만, 안전성이 자동으로 생기지는 않는다. 안전 경계는 API가 아니라 다음에서 나온다.

1. **실행 위치:** host workspace, Git worktree, 누적 diff sandbox, Docker/Kubernetes 중 어디인가.
2. **명령 승인:** 매 명령/파일 변경 승인, rule 기반 허용, 완전 auto-run 중 무엇인가.
3. **복구 단위:** Git commit/diff, 작업별 branch, ephemeral environment, 자동 PR 중 무엇으로 rollback·검토하는가.
4. **비용/신뢰성:** provider API key·rate limit·모델 fallback·장시간 task retry를 제품이 어떻게 다루는가.

따라서 기존 모니터링 조사와 결합할 경우, 직접 API agent를 실행 계층으로, CCC/abtop/Orca류를 상태 관찰·병렬 운영 계층으로 두는 구성이 가능하다. 다만 모니터가 해당 직접 API agent의 event format을 이해하지 못하면, hook·OpenTelemetry·자체 event log 같은 상태 수집 계약을 별도로 만들어야 한다.

## 출처·재현

- 프로젝트 기능, 모델/권한/격리 설명은 각 항목의 공식 GitHub README·공식 문서·링크한 소스 파일만 사용했다.
- star 수 재현: `gh api repos/<owner>/<repo> --jq '.stargazers_count'`.
- ‘직접 API’ 판정은 현재의 설치/로그인 경로 전체를 단정하는 뜻이 아니다. 각 프로젝트에 **API key 또는 endpoint를 주어 자체 agent loop가 provider completion을 호출하는 공식 경로가 존재하는지**로 판정했다.
