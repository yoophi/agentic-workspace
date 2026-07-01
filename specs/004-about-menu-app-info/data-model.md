# Data Model: About 메뉴 앱 정보 표시

## App Information

실행 중인 앱을 식별하기 위해 About 창에 표시되는 읽기 전용 정보 묶음이다.

### Fields

- `appName`: 사용자에게 표시되는 앱 이름.
- `version`: 공식 앱 버전. 배포 산출물과 비교 가능한 값이어야 한다.
- `commitHash`: 빌드 commit hash. 확인할 수 없으면 fallback 값을 사용한다.
- `commitFallback`: commit hash를 확인할 수 없을 때 표시되는 명확한 값.

### Validation Rules

- `appName`은 비어 있으면 안 된다.
- `version`은 비어 있으면 안 되며, 확인할 수 없는 경우에도 명확한 fallback 값이 필요하다.
- `commitHash`는 비어 있거나 공백만 있으면 fallback 값으로 대체한다.
- 표시 라벨은 dev build와 release build에서 동일해야 한다.

### Relationships

- `About Dialog`가 `App Information`을 표시한다.
- `About Entry Point`는 `About Dialog`를 여는 사용자 진입점이다.

## About Entry Point

사용자가 앱 정보를 열기 위해 선택하는 native menu 항목이다.

### Fields

- `label`: 사용자에게 보이는 메뉴 항목 이름.
- `placement`: 플랫폼별 메뉴 위치.
- `enabled`: 사용자가 선택 가능한 상태인지 여부.

### Validation Rules

- `label`은 About 기능임을 명확히 나타내야 한다.
- `placement`는 플랫폼 관례를 따라야 한다.
- 앱 실행 중에는 기본적으로 선택 가능해야 한다.

### State Transitions

```text
Visible -> Selected -> About Dialog Opened -> Closed -> Visible
```

## About Dialog

앱 이름, 버전, commit 정보를 표시하는 작은 읽기 전용 창 또는 dialog다.

### Fields

- `title`: About 창 제목.
- `content`: `App Information`의 표시 텍스트.
- `closeAction`: 사용자가 창을 닫는 동작.

### Validation Rules

- `title`은 앱 정보 창임을 명확히 나타내야 한다.
- `content`에는 앱 이름, 버전, commit 정보가 포함되어야 한다.
- 창을 열거나 닫아도 현재 작업 화면, run, session, prompt draft, permission 상태가 변경되면 안 된다.

### State Transitions

```text
Closed -> Open -> Closed
```
