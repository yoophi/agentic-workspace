# 데이터 모델: Workspace 레이아웃 설정

## WorkspaceLayoutSettings

| 필드 | 형식 | 설명 | 검증 |
|---|---|---|---|
| `workingDirectory` | 문자열 | 설정을 소유하는 Worktree의 정규화된 경로 | 공백만으로 구성될 수 없음 |
| `outerPanelWidthPx` | 선택 정수 | Worktree Session에서 오른쪽 Workspace B의 선호 폭 | 양의 유한 정수일 때만 저장 |
| `panelWidthsPx.git` | 선택 정수 | Git 내부 오른쪽 B의 선호 폭 | 양의 유한 정수일 때만 저장 |
| `panelWidthsPx.files` | 선택 정수 | Files 내부 오른쪽 B의 선호 폭 | 양의 유한 정수일 때만 저장 |
| `panelWidthsPx.markdown` | 선택 정수 | Markdown 내부 오른쪽 B의 선호 폭 | 양의 유한 정수일 때만 저장 |
| `panelWidthsPx.speckit` | 선택 정수 | Speckit 내부 오른쪽 B의 선호 폭 | 양의 유한 정수일 때만 저장 |

## 관계와 소유권

- 하나의 `workingDirectory`는 정확히 하나의 `WorkspaceLayoutSettings` 레코드를 소유한다.
- 한 레코드의 외부 B 폭은 Worktree Session 바깥 분할에 적용한다.
- 패널 종류별 내부 B 폭은 해당 종류의 패널이 표시되고 내부 분할이 존재할 때만 적용한다.
- A 폭과 패널 선택 상태는 이 영속 모델에 저장하지 않는다. A는 현재 컨테이너의 남은 공간을 사용하고, 선택 상태는 현재 화면 수명 동안만 유지한다.

## 상태 전이

```text
설정 없음
  └─(사용자가 B 크기 조절)→ 해당 B 폭만 가진 설정 생성
설정 존재
  └─(같은 Worktree에서 B 재조절)→ 해당 B 폭만 교체
설정 존재 + 작은 화면
  └─(화면 표시)→ 표시 폭만 안전 범위로 제한, 저장된 선호 폭은 유지
```

## 정규화 규칙

1. Worktree 경로는 앞뒤 공백을 제거한 뒤 빈 값이면 거부한다.
2. 폭은 양의 유한 정수만 유지하고, 그 외 값은 저장하지 않는다.
3. 저장된 선호 폭은 화면 크기에 맞춰 영구적으로 변경하지 않는다.
4. 같은 Worktree의 저장은 기존 레코드를 교체하며, 다른 Worktree 레코드는 변경하지 않는다.
