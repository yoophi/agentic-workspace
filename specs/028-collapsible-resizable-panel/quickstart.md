# Quickstart: 접기 및 크기 조절이 가능한 패널 검증

## Prerequisites

- 저장소 루트에서 `pnpm install`이 완료되어 있어야 한다.
- 구현은 [UI contract](./contracts/ui-component.md)와 [data model](./data-model.md)을 충족해야 한다.

## 1. 공용 패키지 단위 검증

```sh
pnpm --filter @yoophi/ui test
pnpm --filter @yoophi/ui check-types
```

Expected:

- 패널별 너비 보존, 둘 다 접힘 방지, separator 활성 파생 상태, 경계 clamp 테스트가 통과한다.
- 공개 props 및 `react-resizable-panels` v4 조합의 타입 오류가 없다.

## 2. 소비 앱 정적 검증

```sh
pnpm --filter @yoophi/git-explorer check-types
pnpm --filter @yoophi/git-explorer build-storybook
```

Expected:

- `@yoophi/ui/components/collapsible-resizable-panels` import가 해석된다.
- Storybook 정적 빌드가 오류 없이 완료된다.

## 3. Storybook 실행

```sh
pnpm run storybook:git
```

`Atomic Design/Molecules/Registered Components`에서 접기·크기 조절 패널 스토리를 연다.

## 4. Manual Scenarios

### Both open

1. Panel A와 Panel B의 제목과 콘텐츠가 모두 보이는지 확인한다.
2. 세로 separator를 좌우로 drag하고 양쪽 너비가 반대로 변하는지 확인한다.
3. 키보드로 separator를 조절하고 focus 표시와 방향이 올바른지 확인한다.

### Panel A collapsed

1. Panel A 제목을 활성화한다.
2. 왼쪽 rail의 우측 끝에 정렬된 −90도 회전 제목과 확장된 Panel B를 확인한다.
3. separator drag가 너비를 바꾸지 않는지 확인한다.
4. Panel B 제목이 disabled인지 확인한다.
5. 왼쪽 rail의 상단, 중앙, 하단을 각각 활성화하고 어느 위치에서든 접기 직전 너비가 복원되는지 확인한다.

### Panel B collapsed

1. Panel B 제목을 활성화한다.
2. 오른쪽 rail의 우측 끝에 정렬된 −90도 회전 제목과 확장된 Panel A를 확인한다.
3. separator 비활성과 Panel A 제목 disabled를 확인한다.
4. 오른쪽 rail의 제목 텍스트가 아닌 빈 공간을 활성화해 접기 직전 너비를 복원한다.

### Boundaries and content

1. 최소·최대 너비를 넘어 separator를 drag해 경계에서 멈추는지 확인한다.
2. 긴 제목이 펼침 및 회전 상태에서 레이아웃 밖으로 넘치지 않는지 확인한다.
3. 빈 콘텐츠와 긴 콘텐츠에서도 제목과 separator가 접근 가능한지 확인한다.
4. 브라우저 너비를 줄인 뒤 접힌 패널을 펼쳐 현재 가용 범위로 clamp되는지 확인한다.

## 5. Accessibility Review

- title button의 accessible name, `aria-expanded`, `aria-controls`, disabled 상태를 확인한다.
- 회전 제목이 보조 기술에서 정상 순서와 이름으로 읽히는지 확인한다.
- 최소 상태 rail 전체에 단일 focus 표시가 나타나고 내부 제목 텍스트가 별도 tab stop을 만들지 않는지 확인한다.
- Storybook a11y 패널에서 critical/serious 위반이 없는지 확인한다.
- 포인터 없이 모든 허용 상태 전환과 separator resize를 완료한다.

## 6. Regression

기존 `ResizableColumns` Story와 Git Explorer의 repository/changes resizable 화면을 열어 기존 `ResizablePanelGroup`, `ResizablePanel`, `ResizableHandle` 동작이 유지되는지 확인한다.
