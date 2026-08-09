# Feature Specification: Markdown Annotator 독립 제품화

**Feature Branch**: `[036-ma-standalone-product]`

**Created**: 2026-08-02

**Status**: Draft

**Input**: User description: "기존 조사와 제품 결정 인터뷰를 반영해 Markdown Annotator를 로컬 Markdown 문서 탐색, annotation, 안전한 복원과 피드백 내보내기가 완결된 macOS 독립 제품으로 만든다."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 폴더에서 Markdown 문서 탐색 (Priority: P1)

사용자는 로컬 폴더를 열고 그 안의 Markdown 문서를 검색 가능한 트리로 탐색하여 원하는 문서를 읽는다. 개발자는 Spec·설계·작업 문서를 찾을 수 있고, 비개발 사용자도 Git이나 에이전트 개념을 알지 않고 일반 Markdown 문서를 찾을 수 있다.

**Why this priority**: 독립 제품은 특정 파일 하나를 미리 아는 경우뿐 아니라 폴더에서 문서를 찾는 흐름을 스스로 완결해야 한다.

**Independent Test**: Markdown 문서와 일반 파일이 섞인 폴더를 열어 Markdown 문서만 검색·선택하고 본문을 읽을 수 있는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 여러 깊이의 폴더에 Markdown과 비-Markdown 파일이 섞여 있을 때, **When** 사용자가 root 폴더를 열면, **Then** Markdown 파일과 그 조상 폴더만 점진적으로 나타난다.
2. **Given** `b/b1/file2.md`가 있고 `b`에는 직접 Markdown 파일이 없을 때, **When** 트리를 표시하면, **Then** 단일 자식 디렉터리 chain이 `b/b1`로 압축되어 `file2.md`의 존재를 보여준다.
3. **Given** Markdown 후손이 없는 `c` 폴더가 있을 때, **When** 스캔이 완료되면, **Then** `c`는 트리에 나타나지 않는다.
4. **Given** 스캔이 진행 중일 때, **When** 첫 문서 batch가 발견되면, **Then** 사용자는 전체 스캔 완료를 기다리지 않고 문서를 선택할 수 있다.
5. **Given** 사용자가 파일명 또는 상대 경로 일부를 입력했을 때, **When** 검색 결과가 표시되면, **Then** 일치 부분과 필요한 조상 폴더가 함께 표시된다.
6. **Given** 사용자가 이름·경로·수정 시각 정렬을 선택했을 때, **When** 같은 root를 다시 열면, **Then** 선택한 정렬이 복원된다.

---

### User Story 2 - 문서에 구조화된 피드백 작성 (Priority: P1)

사용자는 렌더링된 Markdown의 텍스트 구간 또는 블록을 선택해 수정 요청, 질문, 메모 또는 삭제 요청을 작성하고 상태를 관리한다.

**Why this priority**: 읽기만 제공하면 범용 Markdown viewer와 차별화되지 않으며, 구조화된 annotation이 제품의 핵심 가치다.

**Independent Test**: 한 문서에서 네 유형의 annotation을 생성·수정·해결·삭제하고 선택 구간으로 다시 이동할 수 있는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 문서의 텍스트를 선택했을 때, **When** 사용자가 annotation 유형과 내용을 입력하면, **Then** 선택 원문·위치·유형·comment가 하나의 피드백으로 저장된다.
2. **Given** 여러 블록에 걸친 선택이 있을 때, **When** annotation을 생성하면, **Then** 하나의 그룹으로 표시하고 함께 수정·삭제할 수 있다.
3. **Given** 열린 annotation이 있을 때, **When** 사용자가 해결 처리하면, **Then** annotation은 보존되지만 기본 내보내기 범위에서 제외된다.
4. **Given** 사용자가 문서를 `approved`로 결정한 뒤 새 annotation을 추가할 때, **When** 저장이 완료되면, **Then** 문서 결정은 다시 `draft`가 된다.
5. **Given** 열린 수정 또는 삭제 요청이 남아 있을 때, **When** 사용자가 승인을 선택하면, **Then** 경고 후 명시적으로 계속한 경우에만 승인된다.

---

### User Story 3 - 문서를 오가며 작업 복원 (Priority: P1)

사용자는 같은 root의 여러 문서를 오가거나 앱을 종료한 뒤 다시 열어도 문서별 annotation과 읽기 상태를 잃지 않는다.

**Why this priority**: 폴더 browsing을 추가한 상태에서 문서 전환이 피드백 유실로 이어지면 독립 제품으로 신뢰할 수 없다.

**Independent Test**: 문서 A에 annotation을 남기고 B로 이동한 뒤 A로 돌아오며, 앱 재실행 뒤에도 A의 상태가 복원되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 문서 A에 저장된 annotation이 있을 때, **When** B를 열었다가 A로 돌아오면, **Then** A의 annotation, 읽기 위치와 review 상태가 복원된다.
2. **Given** 앱이 정상 또는 비정상 종료되었을 때, **When** 사용자가 시작 화면에서 최근 root와 문서를 다시 열면, **Then** 마지막으로 저장된 review 상태가 복원된다.
3. **Given** 사용자가 앱 아이콘으로 직접 실행할 때, **When** 앱이 시작되면, **Then** root를 자동 복원·스캔하지 않고 시작 화면만 표시한다.
4. **Given** 저장소 일부가 손상되었을 때, **When** 앱을 시작하면, **Then** 앱은 시작 실패하지 않고 복구 또는 초기화 선택지를 제공한다.

---

### User Story 4 - 외부 변경에도 피드백 안전성 유지 (Priority: P1)

사용자는 다른 프로그램에서 원문을 수정·이동·삭제하더라도 annotation이 잘못된 문서나 구간에 조용히 결합되지 않는다는 확신을 가진다.

**Why this priority**: MA는 원문 편집 대신 외부 프로그램과 연계하므로 외부 변경 처리가 핵심 신뢰 조건이다.

**Independent Test**: 원문 블록 이동, 중복 문구 추가, 파일 rename과 삭제를 수행해 자동 재결합·충돌·고아·missing 상태를 확인한다.

**Acceptance Scenarios**:

1. **Given** 현재 문서가 외부에서 수정되었을 때, **When** 변경이 감지되면, **Then** 문서는 자동으로 다시 표시되고 고신뢰 단일 일치 annotation만 재결합된다.
2. **Given** annotation 위치 후보가 여러 개이거나 없을 때, **When** 문서를 다시 읽으면, **Then** 해당 annotation은 충돌 또는 고아 상태로 보존되고 사용자에게 표시된다.
3. **Given** 파일이 같은 root 안에서 rename 또는 이동되고 동일한 단일 후보가 있을 때, **When** 변경을 감지하면, **Then** 사용자에게 재연결을 제안하고 확인 전까지 missing 상태를 유지한다.
4. **Given** 파일이 삭제되거나 읽기 권한을 잃었을 때, **When** 변경을 감지하면, **Then** 마지막 읽기 화면과 review 데이터는 보존되고 복구 action을 제공한다.

---

### User Story 5 - 피드백 검토 및 내보내기 (Priority: P1)

사용자는 현재 문서의 열린 annotation과 문서 결정을 검토한 뒤 사람이 읽을 수 있는 Markdown 또는 안정된 구조의 JSON으로 외부에 전달한다.

**Why this priority**: annotation이 외부 작업으로 이어져야 read-and-review 흐름이 완결된다.

**Independent Test**: annotation별 포함 여부를 선택하고 Markdown과 JSON을 복사·저장해 내용과 schema를 검증한다.

**Acceptance Scenarios**:

1. **Given** 현재 문서에 열린 annotation이 있을 때, **When** 내보내기 화면을 열면, **Then** 열린 annotation 전체가 기본 선택되고 해결된 annotation은 제외된다.
2. **Given** 사용자가 일부 annotation을 제외했을 때, **When** Markdown 또는 JSON을 생성하면, **Then** 선택한 피드백·문서 결정·추가 지침만 포함된다.
3. **Given** annotation 없이 문서 승인만 있을 때, **When** 내보내기를 실행하면, **Then** 승인 결정만 포함한 결과를 만들 수 있다.
4. **Given** 같은 입력 상태일 때, **When** JSON을 여러 번 생성하면, **Then** 동일한 schema version과 결정적인 의미의 결과를 제공한다.
5. **Given** clipboard 쓰기가 실패했을 때, **When** 사용자가 내보내기를 실행하면, **Then** 파일 저장 또는 직접 선택 가능한 결과를 제공한다.

---

### User Story 6 - CLI와 외부 앱으로 연결 (Priority: P2)

사용자는 terminal, Finder와 다른 편집기를 오가며 MA를 읽기·review 중심 도구로 사용한다.

**Why this priority**: MA가 원문을 편집하지 않으므로 기존 파일 작업 도구와의 연결이 필수 편의 기능이다.

**Independent Test**: CLI로 파일과 디렉터리를 열고, 현재 문서를 Finder와 기본 앱에서 열며 외부 수정 자동 반영을 확인한다.

**Acceptance Scenarios**:

1. **Given** CLI가 설치되어 있을 때, **When** `ma directory/` 또는 `ma .`을 실행하면, **Then** 해당 canonical root의 창을 열거나 기존 창을 포커스한다.
2. **Given** Markdown 파일 경로가 있을 때, **When** `ma file.md`를 실행하면, **Then** 부모 폴더를 root로 열고 해당 파일을 선택한다.
3. **Given** 인자 없이 CLI를 실행할 때, **When** 현재 디렉터리를 해석할 수 있으면, **Then** 현재 디렉터리를 root로 연다.
4. **Given** 현재 문서가 존재할 때, **When** Finder에서 보기·기본 앱으로 열기·경로 복사를 선택하면, **Then** 요청한 외부 action을 수행한다.
5. **Given** 사용자가 CLI 설치 또는 제거를 선택할 때, **When** 작업이 완료되면, **Then** 대상 위치와 PATH 조건 및 현재 상태를 명확히 표시한다.

---

### User Story 7 - 설정과 로컬 데이터 통제 (Priority: P2)

사용자는 전역 제외 디렉터리와 문서 글꼴 크기를 조정하고, MA가 보관하는 최근 기록·review·backup 데이터를 확인하고 삭제한다.

**Why this priority**: 임의의 로컬 폴더와 민감한 review context를 다루는 제품은 탐색 범위와 데이터 보존을 사용자가 통제하게 해야 한다.

**Independent Test**: 제외 목록 변경·즉시 재스캔, 글꼴 크기 변경, root별 데이터 삭제와 전체 초기화를 수행한다.

**Acceptance Scenarios**:

1. **Given** 기본 제외 목록이 있을 때, **When** 사용자가 디렉터리 이름을 추가·삭제하면, **Then** 열린 모든 창이 기존 화면을 유지한 채 비파괴적으로 다시 스캔한다.
2. **Given** 현재 문서가 새 제외 규칙에 해당할 때, **When** 재스캔되면, **Then** 문서를 강제로 닫지 않고 browser 범위에서 제외됐음을 표시한다.
3. **Given** 사용자가 문서 글꼴 크기를 변경할 때, **When** 다른 root나 앱 재실행 후 문서를 열면, **Then** 전역 설정이 적용된다.
4. **Given** 사용자가 특정 문서·root 또는 전체 app-data 삭제를 선택할 때, **When** 범위를 확인하면, **Then** 원본 Markdown에는 영향 없이 해당 로컬 데이터만 삭제된다.
5. **Given** 삭제 데이터가 휴지통 보존 기간 안에 있을 때, **When** 사용자가 복원을 선택하면, **Then** review 데이터를 되돌릴 수 있다.

---

### User Story 8 - 제품 및 개인정보 확인 (Priority: P3)

사용자는 About 화면에서 제품 목적, 버전, 빌드, 지원 형식, 로컬 데이터 원칙과 법적 정보를 확인한다.

**Why this priority**: 독립 설치 제품은 문제 보고와 신뢰 판단에 필요한 기본 정보를 스스로 제공해야 한다.

**Independent Test**: native About 메뉴에서 별도 화면을 열어 모든 필수 정보와 fallback 상태를 확인한다.

**Acceptance Scenarios**:

1. **Given** 앱이 실행 중일 때, **When** 사용자가 About 메뉴를 선택하면, **Then** 제품명·설명·release version·commit·tag를 확인할 수 있다.
2. **Given** build metadata 일부를 알 수 없을 때, **When** About을 열면, **Then** 알 수 없는 값이 명시되고 화면은 정상 동작한다.
3. **Given** 사용자가 데이터 처리와 법적 정보를 찾을 때, **When** About을 열면, **Then** local-first 안내, 지원 형식, 라이선스와 third-party notices 진입점을 확인할 수 있다.

### Edge Cases

- root에 Markdown 파일이 하나도 없으면 빈 상태와 다른 폴더·파일 열기 action을 제공한다.
- 접근할 수 없는 하위 폴더는 전체 스캔을 실패시키지 않고 건너뛴 경로 수와 경고를 제공한다.
- 전역 제외 이름에는 빈 값, root, `..` 또는 경로 separator를 허용하지 않는다.
- root 밖을 가리키는 symlink는 표시·읽기하지 않는다. root 내부 파일 symlink는 중복과 loop 없이 표시하며 directory symlink는 따라가지 않는다.
- Markdown이 아닌 파일과 `.mdx`는 MA 트리에 표시하지 않는다.
- UTF-8 또는 UTF-8 BOM으로 해석할 수 없는 파일은 대체 문자로 조용히 표시하지 않고 오류와 외부 앱 action을 제공한다.
- raw HTML, script, iframe, event handler와 실행 가능한 콘텐츠는 실행하지 않는다.
- 외부 이미지는 자동 로드하지 않고, root 밖 local asset은 접근하지 않는다.
- 같은 문서 read 요청이 경쟁하면 가장 최근 선택만 화면과 review 상태를 변경한다.
- 검색 중 일치 결과의 조상은 임시 확장되고 검색 종료 후 이전 펼침 상태로 돌아간다.
- 저장 용량 상한에 가까워지면 active review를 삭제하지 않고 오래된 backup과 휴지통부터 정리한다.
- 앱 아이콘 직접 실행은 최근 root를 자동 스캔하지 않고 시작 화면만 표시한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 사용자가 로컬 디렉터리 하나를 창의 root로 열 수 있게 해야 한다.
- **FR-002**: 시스템은 파일 하나를 열 때 부모 디렉터리를 root로 사용하고 해당 파일을 즉시 선택해야 한다.
- **FR-003**: 시스템은 같은 canonical root를 중복 창으로 열지 않고 기존 창을 포커스해야 한다.
- **FR-004**: 시스템은 root를 백그라운드 전체 스캔하고 발견한 Markdown 파일을 점진적으로 표시해야 한다.
- **FR-005**: 시스템은 `.md`와 `.markdown`만 문서 대상으로 표시하고 `.mdx`와 비-Markdown 파일은 표시하지 않아야 한다.
- **FR-006**: 시스템은 Markdown 후손이 있는 모든 조상 디렉터리를 표시하고 후손이 없는 branch는 숨겨야 한다.
- **FR-007**: 시스템은 파일 없이 단일 디렉터리만 이어지는 chain을 하나의 경로 row로 압축해야 한다.
- **FR-008**: 사용자는 파일명과 상대 경로를 대소문자 비구분 substring으로 검색할 수 있어야 한다.
- **FR-009**: 사용자는 이름·경로·수정 시각으로 정렬할 수 있어야 하며 디렉터리 우선 natural name 정렬이 기본이어야 한다.
- **FR-010**: 시스템은 root별 정렬·펼침 상태를 보존하고 active 문서의 조상을 자동 확장해야 한다.
- **FR-011**: 시스템은 전역 제외 디렉터리 이름 목록과 기본값 복원을 제공해야 한다.
- **FR-012**: 제외 설정 변경은 열린 창에 즉시 전달되고 기존 문서와 review를 잃지 않는 재스캔으로 적용되어야 한다.
- **FR-013**: 시스템은 root 밖 경로, traversal과 root 밖 symlink 접근을 차단해야 한다.
- **FR-014**: 시스템은 root 내부 file symlink만 허용하고 directory symlink는 따라가지 않아야 한다.
- **FR-015**: 시스템은 읽을 수 없는 디렉터리를 건너뛰면서 접근 가능한 결과를 계속 제공해야 한다.
- **FR-016**: 시스템은 현재 문서와 트리의 생성·삭제·rename·수정 변경을 감지해야 한다.
- **FR-017**: 외부에서 현재 문서가 수정되면 시스템은 자동으로 다시 표시하고 annotation 재결합 결과를 알려야 한다.
- **FR-018**: 시스템은 같은 root에서 파일이 이동됐을 가능성이 있는 단일 고신뢰 후보를 제안하되 사용자 확인 없이 review를 재연결하지 않아야 한다.
- **FR-019**: 시스템은 충돌·고아·missing annotation과 review를 삭제하지 않고 사용자가 재연결 또는 폐기할 때까지 보존해야 한다.
- **FR-020**: 사용자는 선택 영역 또는 블록에 수정 요청, 질문, 메모와 삭제 요청 annotation을 작성할 수 있어야 한다.
- **FR-021**: 시스템은 여러 블록 annotation을 하나의 그룹으로 생성·수정·삭제할 수 있게 해야 한다.
- **FR-022**: 사용자는 annotation을 open 또는 resolved로 관리할 수 있어야 한다.
- **FR-023**: 사용자는 문서 review를 draft, changes-requested, approved 또는 stopped로 결정할 수 있어야 한다.
- **FR-024**: 열린 수정·삭제 요청이 있는 승인과 승인 후 새 annotation 작성에는 명시적 경고 또는 상태 전환이 적용되어야 한다.
- **FR-025**: 시스템은 문서별 annotation, review 결정, export 설정과 읽기 위치를 자동 저장해야 한다.
- **FR-026**: 저장은 원본 폴더에 sidecar를 만들지 않고 앱의 로컬 데이터 영역에 유지되어야 한다.
- **FR-027**: 시스템은 저장 schema version과 migration, atomic save와 손상 복구를 제공해야 한다.
- **FR-028**: 시스템은 최근 snapshot 5개와 삭제된 review 데이터의 7일 복구를 제공해야 한다.
- **FR-029**: 시스템은 app-data를 100MB 목표 안에서 관리하되 active review 데이터를 자동 삭제하지 않아야 한다.
- **FR-030**: 사용자는 최근 폴더·문서 기록, 특정 문서·root review와 모든 app-data를 선택적으로 삭제할 수 있어야 한다.
- **FR-031**: 시스템은 현재 문서의 open annotation 전체를 기본으로 Markdown과 schema-versioned JSON 피드백을 생성해야 한다.
- **FR-032**: 사용자는 내보낼 annotation을 개별 선택하고 resolved annotation을 명시적으로 포함할 수 있어야 한다.
- **FR-033**: 시스템은 Markdown과 JSON 결과를 clipboard 또는 UTF-8 파일로 제공해야 한다.
- **FR-034**: JSON v1은 문서 identity, review 결정, 선택 annotation과 schema version을 손실 없이 표현해야 한다.
- **FR-035**: 시스템은 여러 문서를 한 번에 내보내지 않고 현재 문서 단위로 결과를 생성해야 한다.
- **FR-036**: 시스템은 같은 root 내부의 Markdown 상대 링크와 wikilink를 내부 이동으로 처리하고 heading fragment로 이동해야 한다.
- **FR-037**: 시스템은 HTTP/HTTPS 링크를 명시적 사용자 action으로 외부 browser에서 열고 root 밖 local 링크를 차단해야 한다.
- **FR-038**: 시스템은 단일 active 문서, 뒤로·앞으로 navigation과 최근 문서를 제공하고 문서 탭은 제공하지 않아야 한다.
- **FR-039**: 사용자는 현재 문서를 Finder에서 표시하고 기본 앱으로 열며 절대 경로를 복사할 수 있어야 한다.
- **FR-040**: CLI는 디렉터리, 현재 디렉터리 또는 Markdown 파일을 열 수 있어야 하며 여러 경로·glob·stdin·headless 처리는 제공하지 않아야 한다.
- **FR-041**: 사용자는 관리자 권한 없이 CLI를 명시적으로 설치·확인·재설치·제거할 수 있어야 한다.
- **FR-042**: 앱 아이콘 직접 실행은 root 자동 복원 없이 시작 화면을 표시해야 한다.
- **FR-043**: 시작 화면은 최근 폴더·문서, 폴더 열기, 파일 열기와 간단한 3단계 사용 안내를 제공해야 한다.
- **FR-044**: 제품 UI와 배포 산출물은 내장 예제 browser를 제공하지 않아야 한다.
- **FR-045**: 기본 화면은 파일 tree, 문서, review의 세 영역을 제공하고 목차는 접을 수 있어야 한다.
- **FR-046**: 사용자는 좌우 영역을 숨기거나 문서 집중 모드로 전환할 수 있어야 한다.
- **FR-047**: 시스템은 키보드만으로 tree 탐색, 문서 이동, annotation 작성과 export가 가능해야 한다.
- **FR-048**: 전역 설정은 제외 디렉터리와 문서 글꼴 크기만 제공하고 테마 설정은 포함하지 않아야 한다.
- **FR-049**: About 화면은 제품 정보, CALVER, commit, tag, 지원 형식, local-first 원칙, 라이선스와 notices를 표시해야 한다.
- **FR-050**: 시스템은 자동 telemetry나 crash report 전송을 하지 않고 redaction된 local 진단 정보만 사용자 요청으로 제공해야 한다.
- **FR-051**: 시스템은 UTF-8과 UTF-8 BOM만 공식 지원하고 해석 실패를 명확히 알려야 한다.
- **FR-052**: 시스템은 raw HTML과 실행 가능한 콘텐츠를 실행하지 않고 root 밖 local asset과 자동 외부 이미지 로드를 차단해야 한다.
- **FR-053**: 첫 안정 릴리스의 사용자 UI는 한국어로 일관되게 제공되어야 한다.
- **FR-054**: 첫 안정 릴리스는 macOS 설치·native 메뉴·Finder·CLI·서명된 배포 흐름을 공식 지원해야 한다.
- **FR-055**: 자동 업데이트, 원문 편집, 파일 생성·rename·이동·삭제, 즐겨찾기, command palette와 사용자 지정 외부 앱은 제공하지 않아야 한다.
- **FR-056**: Spec·Task 관계 모델, Artifact·Workflow 연결, 앱 내부 에이전트 실행과 MA↔AW 직접 전송은 첫 안정 릴리스에 포함하지 않아야 한다.
- **FR-057**: 공통 파일 tree 동작은 범용 파일 entry를 다루고 MA는 Markdown scope, AW는 제품별 scope를 구성할 수 있어야 한다.
- **FR-058**: 공통 파일 tree 동작은 검색·정렬·경로 압축·visible row·keyboard와 접근성 의미를 두 앱에 일관되게 제공해야 한다.
- **FR-059**: 파일 watcher, 앱별 상태 수명, recent root, worktree·SpecKit와 annotation 연결은 각 제품이 독립적으로 소유해야 한다.
- **FR-060**: MA는 AW release와 독립적인 CALVER release candidate와 stable artifact를 만들 수 있어야 한다.
- **FR-061**: 첫 안정 릴리스는 수동 업데이트만 지원해야 한다.

### Key Entities

- **Root Folder**: 한 창이 탐색하는 canonical 로컬 디렉터리. 표시명, 최근 사용 시각, tree view 상태와 제외 정책 적용 결과를 가진다.
- **File Entry**: root-relative 파일 또는 디렉터리 항목. 상대 경로, 종류, 수정 시각, 크기와 symlink 여부를 가진다.
- **Document Identity**: canonical root, 상대 경로와 content fingerprint를 결합해 review가 속한 문서를 식별한다.
- **Review Session**: 문서 하나에 대한 annotation, 문서 결정, export 설정, 읽기 위치와 저장 schema version의 집합이다.
- **Annotation**: 문서 구간에 대한 수정 요청·질문·메모·삭제 요청. anchor, 선택 원문, comment, open/resolved 상태와 재결합 상태를 가진다.
- **Review Decision**: 문서 단위 draft, changes-requested, approved 또는 stopped 상태와 변경 시각이다.
- **Feedback Export**: 현재 문서 review에서 사용자가 선택한 annotation과 결정을 Markdown 또는 JSON v1로 표현한 결과다.
- **Global Preferences**: 제외 디렉터리 이름 목록과 문서 글꼴 크기다.
- **Recent Location**: 사용자가 명시적으로 다시 열 수 있는 최근 root와 문서 기록이다.
- **Build Information**: 제품명, release version, commit, tag와 법적·지원 정보다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: MA 제품 orchestration은 `apps/markdown-annotator`에 한정한다. AW와 MA가 함께 소비하는 범용 파일 tree 계산과 검증된 tree interaction만 `packages/*`의 공통 module로 둔다. 앱 간 직접 import는 허용하지 않는다. 안전한 filesystem 공통화는 이번 feature가 아니라 후속 검증 대상으로 둔다.
- **Frontend layering**: 앱 조립과 시작·설정·About routing은 `app`/`pages`, folder browsing과 review action은 `features`, file·review·preference 모델과 adapter는 `entities`, 앱 전용 공통 표현은 `shared`, 생성 UI primitive는 `components/ui`에 둔다.
- **Backend boundary**: 문서·review·설정 모델과 순수 규칙은 domain, 유스케이스는 application, Tauri command와 native menu event는 inbound, filesystem·watcher·JSON·CLI·OS 연계는 infrastructure에 둔다. MA는 port 위치를 한곳에서 일관되게 사용한다.
- **Shared core vs UI**: 범용 tree의 순수 entry→row 계산을 먼저 공유하고 AW를 첫 소비자로 검증한 뒤 MA adapter를 추가한다. tree keyboard·ARIA 요구가 양쪽에서 일치할 때만 kit-independent 공유 UI를 제공하며 앱별 UI primitive를 주입한다. watcher와 상태 orchestration은 공유하지 않는다.
- **Persistence and safety**: review·preference 저장은 repository/port 뒤에 두고 schema migration·atomic save·backup을 제공한다. 파일 접근은 canonical root, traversal, symlink, size와 UTF-8 정책을 강제한다. 민감한 문서 내용과 절대 경로는 자동 telemetry나 기본 진단 로그에 포함하지 않는다.
- **Documentation and Storybook**: 제품·architecture·release·사용자 문서를 한국어 `docs/*.md`로 갱신한다. 공유 file tree와 MA의 시작·tree·review·About·설정 UI는 loading, progress, empty, error, deep tree, long path와 접근성 상태를 Storybook에 등록한다. 예제 Markdown은 제품 UI가 아니라 test/Storybook fixture로만 유지한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 신규 사용자의 90% 이상이 별도 외부 설명 없이 3분 안에 폴더 열기, 문서 찾기, annotation 작성과 Markdown 복사를 완료한다.
- **SC-002**: 10,000개 filesystem entry와 1,000개 Markdown 파일이 있는 일반적인 로컬 SSD root에서 첫 문서 batch가 1초 안에 표시되고 전체 스캔이 5초 안에 완료된다.
- **SC-003**: 파일명·경로 검색, 정렬과 폴더 펼침 결과가 사용자 action 후 100ms 안에 갱신된다.
- **SC-004**: 1MB Markdown 문서가 선택 후 500ms 안에 읽을 수 있는 상태로 표시되며, 5MB 문서 처리 중에도 앱이 사용자 입력에 응답한다.
- **SC-005**: 저장 완료된 annotation의 앱 재실행·문서 왕복 복원 성공률은 100%이고, 다른 문서 또는 잘못된 구간에 자동 결합되는 사례는 검증 fixture에서 0건이다.
- **SC-006**: root 밖 path·traversal·symlink 접근 검증 시나리오에서 root 밖 파일 노출과 읽기가 0건이다.
- **SC-007**: 지원하는 외부 변경 시나리오 전체에서 annotation은 유지·충돌·고아·missing 중 하나의 명시적 상태를 가지며 조용히 삭제되는 사례는 0건이다.
- **SC-008**: 동일 review 상태에서 생성한 JSON v1 결과는 모든 contract fixture에서 schema validation을 통과하고 의미상 동일한 결과를 제공한다.
- **SC-009**: 키보드만 사용하는 검증자가 폴더 탐색, 문서 선택, annotation 작성과 export를 모두 완료하며, VoiceOver가 tree item·선택·펼침·annotation action을 식별한다.
- **SC-010**: clean macOS 환경에서 설치, About, 폴더·파일 열기, Finder·기본 앱 연계, CLI 설치·사용·제거와 수동 업그레이드 acceptance가 모두 통과한다.
- **SC-011**: 공통 파일 tree 변경 후 AW와 MA의 tree contract 및 핵심 사용자 흐름 회귀가 0건이다.
- **SC-012**: 앱은 사용자의 문서 내용, 경로 또는 annotation을 명시적 export action 없이 외부로 전송하지 않는다.

## Assumptions

- 1차 사용자는 AI 에이전트가 읽거나 생성한 Markdown을 검토하는 개발자이며, 2차 사용자는 일반 로컬 Markdown을 읽고 구조화된 피드백을 전달하는 비개발 사용자다.
- 사용자는 원문 편집을 기존 편집기 또는 다른 프로그램에서 수행한다.
- 첫 안정 릴리스의 공식 지원 운영체제는 macOS이며 Windows/Linux는 공식 완료 기준에 포함하지 않는다.
- 사용자는 시스템 계정과 disk 보안 기능을 신뢰하며 앱 자체 저장 암호화는 요구하지 않는다.
- 기본 제외 디렉터리는 `.git`, `node_modules`, `target`, `dist`, `build`, `.next`이고 사용자가 전역 목록을 변경할 수 있다.
- app-data는 원문 전체를 복제하지 않고 annotation 원문·주변 context와 review 상태만 저장한다.
- 최근 항목은 명시적 navigation 편의 기능이며 앱 직접 실행 시 자동 복원 근거로 사용하지 않는다.
- JSON v1은 향후 AW 또는 workflow 소비자가 사용할 수 있지만 이번 feature에는 직접 앱 간 전달이 포함되지 않는다.
- rendering 검증용 Markdown 자료는 자동 test와 Storybook fixture로 유지하되 실제 제품 browser에는 노출하지 않는다.
- 공통 file browser module 명칭은 `file-browser-core`와 `file-browser-react`이며, 안전한 filesystem 공통 crate는 후속 작업에서 검토한다.
