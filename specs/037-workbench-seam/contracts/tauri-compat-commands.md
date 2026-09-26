# Contract: Tauri compat commands (037)

**Spec**: [../spec.md](../spec.md) FR-002 · **Research**: [../research.md](../research.md) R9

프론트엔드(`apps/agentic-workbench/src/entities/project/api/project-repository.ts`)는 바뀌지 않는다. 아래 두 command는 시그니처·직렬화·오류 문자열이 **바이트 단위로 이전과 같아야** 한다.

## 불변 시그니처

```rust
#[tauri::command] pub fn list_projects(app: AppHandle) -> Result<Vec<Project>, String>
#[tauri::command] pub fn create_project(app: AppHandle, input: ProjectInput) -> Result<Project, String>
```

- `Project`·`ProjectInput`의 JSON 형태 불변(`id`, `name`, `workingDirectory`, `description`).
- 내부는 `app.state::<Arc<WorkbenchRuntime>>()`에서 runtime을 얻어 `workbench_compat::{list_projects, create_project}` 순수 함수로 CallRequest를 만들고 `call`한 뒤 결과를 변환한다.
- `update_project`/`delete_project`는 037에서 operation이 되지 않지만, 저장은 runtime이 소유한 `JsonProjectRepository`와 coordinator lock을 통해 수행한다(R4). 시그니처·오류 문구 불변.

## Fault → String 매핑

| Fault code | 반환 `Err(String)` |
|---|---|
| 모든 코드 | `fault.message` 그대로 |

즉 코드 prefix, JSON 직렬화, 추가 정보를 붙이지 않는다.

## 보존해야 하는 문구(골든)

| 상황 | 문자열 |
|---|---|
| 이름 비어 있음 | `Project name is required.` |
| 작업 디렉터리 비어 있음 | `Working directory is required.` |
| (update/delete) 없는 id | `Project not found.` |
| 저장 파일 읽기/쓰기 실패 | 기존 `json_store` 메시지(`Failed to read projects: …` 등) 그대로 |
| 앱 데이터 디렉터리 해석 실패 | `Failed to resolve app data directory: …`(조립부에서 발생, setup 실패로 승격) |

새로 생기는 실패(ledger 잠김 등)는 한국어 한 문장이며 이 표에 없는 문구가 화면에 새로 나타날 수 있다. 이는 이전 버전에서 존재하지 않던 실패 모드라 FR-002 위반이 아니다.

## 검증

- `apps/agentic-workbench/src-tauri/src/inbound/workbench_compat.rs` 유닛 테스트: fixture(`crates/workbench-protocol/fixtures/`)의 `request.input` → `ProjectInput` → `CallRequest` 변환이 fixture의 `request`와 같은지, `expect.fault.message` → `Err(String)`이 같은지.
- 기존 `project_service` 테스트(있다면)는 core로 이동하되 기대 문자열은 유지.
- 수동: 앱에서 이름 없이 프로젝트 저장 → 화면 오류 문구가 "Project name is required."인지 확인(quickstart §4).
