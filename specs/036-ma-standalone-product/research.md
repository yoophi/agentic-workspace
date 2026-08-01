# Research: Markdown Annotator 독립 제품화

## Shared file browser의 추출 순서

**Decision**: 순수 `file-browser-core`를 먼저 만들고 AW를 첫 소비자로 전환한 뒤 MA adapter를 추가한다. core 계약이 안정된 후 `file-browser-react`를 추출한다.

**Rationale**: AW에 lazy entry merge·visibility fixture가 있고 `packages/git-ui`에 directory 합성·단일 chain 압축 fixture가 있어 실제 동작을 보존하며 일반화할 근거가 있다. constitution의 shared core before shared UI도 만족한다.

**Alternatives considered**: AW feature를 MA에서 직접 import하면 worktree/React Query/agent 문맥이 유출된다. 두 앱이 tree를 따로 구현하면 검색·압축·접근성이 분기된다. Git UI까지 동시에 전환하면 Git status payload 회귀 범위가 커진다.

## File browser core interface

**Decision**: 범용 relative-path entry와 options를 받아 display row를 한 번에 만드는 `createFileBrowserRows` 하나를 주 interface로 둔다. Markdown filter, watcher와 persistence는 앱 adapter가 소유한다.

**Rationale**: caller가 build/filter/sort/compress/flatten 순서를 배울 필요가 없어 interface가 깊어진다. AW의 explicit directory entry와 MA의 file-only progressive batch를 같은 입력으로 정규화할 수 있다.

**Alternatives considered**: 여러 helper를 모두 export하면 호출 순서와 중간 불변식이 앱마다 복제된다. app entity를 generic metadata로 그대로 운반하면 shared module이 제품 payload에 결합된다.

## Directory chain compression과 lazy state

**Decision**: 파일 없이 단일 child directory만 이어지는 chain을 압축하고 마지막 directory path를 toggle/load identity로 사용한다. row는 child state unknown/loading/loaded를 표현할 수 있다.

**Rationale**: MA 요구와 기존 Git UI 동작이 일치하며 깊은 tree 소음을 줄인다. 마지막 path를 사용해야 AW lazy fetch가 실제 펼칠 directory와 일치한다.

**Alternatives considered**: 첫 path를 identity로 쓰면 AW가 중간 빈 directory를 별도 load해야 한다. 항상 압축하지 않으면 확정된 제품 요구를 만족하지 못한다.

## Shared React tree

**Decision**: tree semantics, roving focus, keyboard navigation, active scroll와 virtualization은 shared React package가 소유하고 header·controls·icons·copy는 앱 adapter가 소유한다.

**Rationale**: 접근성과 virtualization focus는 분리하면 회귀하기 쉬우며 두 앱 요구가 동일하다. 반면 AW와 MA의 UI primitive 계열과 loading 의미는 다르다.

**Alternatives considered**: Button/ScrollArea를 직접 의존하면 한 앱 kit가 다른 앱에 유출된다. virtualization을 앱별 구현하면 keyboard focus와 DOM window 계약이 갈라진다.

## MA root scan 전략

**Decision**: blocking worker에서 취소 가능한 전체 scan을 수행하고 count/time batch로 progress를 emit한다. batch에는 Markdown file과 ancestor directory, scan id, sequence와 partial warning이 포함된다.

**Rationale**: Markdown 없는 branch 숨김과 chain compression에는 전체 구조가 필요하지만 first-result latency 때문에 완료를 기다릴 수 없다. stale scan id를 무시하면 setting/root 변경 경쟁도 안전하게 처리한다.

**Alternatives considered**: AW식 lazy scan은 전체 검색 결과와 빈 branch 판단이 불완전하다. main thread scan은 UI를 멈춘다. final snapshot만 반환하면 첫 batch 목표를 만족하지 못한다.

## 파일 범위와 경로 안전성

**Decision**: `.md`/`.markdown`, strict UTF-8(BOM 제거)만 허용한다. root는 한 번 canonicalize하고 relative input의 absolute/`..`/prefix/NUL을 거부한다. directory symlink는 따르지 않고 root 내부 regular Markdown file symlink만 dedupe해 허용한다.

**Rationale**: `.mdx`를 단순 Markdown처럼 렌더하면 안전과 정확성을 과장한다. canonical target과 display path를 분리하면 외부 root 접근과 duplicate identity를 막으면서 내부 alias는 보존할 수 있다.

**Alternatives considered**: hidden directory 전체 제외는 `.github`/`.specify` 문서를 숨긴다. `.gitignore` 적용은 untracked 개인 문서를 숨긴다. directory symlink follow는 loop와 예상 밖 대규모 scan을 만든다.

## Exclusion policy

**Decision**: exact directory name 전역 목록을 모든 깊이에 적용하고 기본값은 `.git`, `node_modules`, `target`, `dist`, `build`, `.next`다. 변경은 모든 창에 broadcast하고 비파괴 rescan한다.

**Rationale**: glob/root별 예외 없이도 주요 비용 경로를 제어하며 사용자가 제품 범위를 이해하기 쉽다. current document가 새로 제외돼도 review를 잃지 않는다.

**Alternatives considered**: 고정 목록만 제공하면 사용자 환경을 반영하지 못한다. glob은 validation과 예측 가능성을 복잡하게 만든다.

## Root watcher

**Decision**: root/window당 recursive watcher 하나와 trailing debounce를 사용한다. platform rename pair를 신뢰하지 않고 event를 snapshot diff/rescan 힌트로 취급한다.

**Rationale**: 현재 MA의 leading-edge 단일 파일 watcher는 atomic save burst의 마지막 event를 잃을 수 있다. tree와 current document가 같은 root revision event를 소비하면 중복 reload를 막는다.

**Alternatives considered**: 단일 파일 watcher와 root watcher를 병렬 유지하면 event와 lifecycle이 중복된다. raw rename event로 identity를 즉시 변경하면 editor/platform 차이로 오결합할 수 있다.

## Review session aggregate

**Decision**: annotation CRUD가 아니라 version/revision이 있는 문서별 `ReviewSession` 전체를 persistence 단위로 둔다.

**Rationale**: decision, export preferences, reading position, fingerprint와 annotation 상태는 함께 일관되어야 한다. expected revision은 autosave와 문서 전환 경쟁에서 stale frontend write를 거부한다.

**Alternatives considered**: annotation별 repository method는 shallow interface와 부분 저장 불변식을 만든다. 하나의 거대 JSON은 단일 손상이 모든 review에 전파되고 scoped delete/quota가 비싸다.

## Atomic storage, migration과 recovery

**Decision**: 세션별 envelope JSON을 unique temp→sync→snapshot rotate→atomic rename→parent sync 순서로 저장한다. explicit sequential migration, corrupt 격리와 newest-valid-snapshot recovery를 사용한다.

**Rationale**: fixed temp는 동시 write 충돌 위험이 있고 unknown future schema를 reset하면 downgrade 시 데이터가 유실된다. 세션별 파일은 손상 격리와 revision conflict 처리가 쉽다.

**Alternatives considered**: localStorage는 filesystem/security/migration control이 약하다. 원본 폴더 sidecar는 Git 변경과 개인정보 노출을 만든다. AW JSON store를 이번에 shared crate로 옮기면 범위가 확대된다.

## Retention과 quota

**Decision**: valid snapshot 최근 5개, trash 7일, app-data 100MB maintenance target을 사용한다. expired trash와 오래된 snapshot부터 정리하고 active review save는 계속 허용한다.

**Rationale**: 복구 가능성과 민감한 context의 제한된 보존을 균형 있게 유지한다. 100MB를 hard limit로 쓰면 active review가 저장되지 않아 핵심 신뢰를 해친다.

**Alternatives considered**: 무제한 backup은 예측 불가능한 민감 데이터 축적을 만든다. active review 자동 삭제는 제품 불변식을 위반한다.

## Annotation 재결합과 file relink

**Decision**: block id 또는 selected text+context의 유일 exact match만 자동 재결합한다. ambiguity는 conflict, 없음은 orphan으로 보존한다. rename/move는 같은 root의 동일 fingerprint 단일 후보를 제안하고 사용자 확인 후 relink한다.

**Rationale**: 잘못된 문서/문단에 조용히 피드백을 붙이는 것이 missing 상태보다 위험하다.

**Alternatives considered**: fuzzy 자동 매칭은 짧거나 반복되는 문구에 오탐을 만든다. path 변경만으로 review를 삭제하면 사용자 작업을 잃는다.

## Feedback export

**Decision**: 사람이 읽는 Markdown과 JSON Schema로 고정한 deterministic JSON v1을 current document 단위로 제공한다.

**Rationale**: 비개발 사용자와 agent workflow 모두 사용할 수 있고 MA가 특정 runtime에 종속되지 않는다. schema version은 향후 AW adapter의 안정 seam이 된다.

**Alternatives considered**: Markdown만 제공하면 구조적 소비가 불안정하다. 앱 내부 agent 실행이나 AW 직접 전송은 독립 제품 완료 기준을 흐린다.

## Native About와 Settings

**Decision**: AW의 native menu/stable window 패턴을 참고하되 About과 Settings는 MA-local 전용 route/window로 구성한다.

**Rationale**: About 요구 정보와 link/notices는 message dialog 용량을 넘는다. stable window는 메뉴 호출마다 중복 생성하지 않고 현재 document window에 종속되지 않는다.

**Alternatives considered**: native message dialog는 링크와 notices interaction이 부족하다. current page modal은 native menu와 multi-window focus가 결합된다.

## Finder와 기본 앱 연계

**Decision**: canonical document를 application service에서 검증한 뒤 macOS adapter가 argument array로 `open -R` 또는 `open`을 실행한다.

**Rationale**: Finder reveal은 generic opener만으로 표현하기 어렵고 raw frontend shell 권한은 경로 안전 seam을 우회한다.

**Alternatives considered**: shell string 실행은 injection 위험이 있다. 사용자 지정 app registry는 첫 릴리스 범위를 넘는다.

## CLI target과 window identity

**Decision**: cold start와 single-instance callback이 같은 resolver를 사용해 `ma [file-or-directory]`를 canonical root+optional document로 변환한다. window identity는 document가 아니라 root다.

**Rationale**: 창당 root 하나·active document 하나라는 제품 결정과 일치하며 동일 root 중복 watcher를 피한다.

**Alternatives considered**: 기존 document hash window/native tab 모델은 folder browsing과 충돌한다. wrapper와 release CLI binary의 독립 resolver는 정책이 분기된다.

## CALVER와 macOS release

**Decision**: release build config overlay와 build env에 같은 CALVER를 주입하고 About/bundle metadata 일치를 검증한다. stable artifact는 Developer ID signing, notarization, stapling과 Gatekeeper 검증을 release blocker로 둔다.

**Rationale**: manifest version 변경 없이 독립 MA release를 만들고 설치된 앱에서 정확한 build 정보를 제공한다.

**Alternatives considered**: package/Cargo/Tauri manifest 직접 변경은 workspace versioning 규칙을 위반한다. runtime git 조회는 설치 앱에서 작동하지 않는다. 자동 updater는 별도 feed/rollback 보안 범위가 필요하다.

## Agent context update

**Decision**: 저장소에 `update-agent-context` script가 존재하지 않아 자동 갱신을 수행하지 않는다. AGENTS.md의 기존 MA/FSD/hexagonal/CALVER 지침과 본 plan을 구현 context로 사용한다.

**Rationale**: `.specify/scripts/bash`에는 setup/check/create scripts만 있으며 없는 command를 임의 생성하거나 대체 실행하면 workflow를 왜곡한다.

**Alternatives considered**: AGENTS.md를 계획 중 자동 수정하는 방법은 이번 feature의 source of truth와 무관한 전역 지침 변경을 만든다.
