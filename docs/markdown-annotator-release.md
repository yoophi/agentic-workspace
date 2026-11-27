# Markdown Annotator macOS 릴리스

릴리스는 `YYYY.M.D-rc.N` 또는 `YYYY.M.D` CALVER를 사용한다. 기본 manifest의 버전은 바꾸지 않고 release overlay와 `MA_CALVER`, `MA_GIT_COMMIT`, `MA_GIT_TAG` 환경 변수로 artifact와 About 정보를 일치시킨다.

```mermaid
flowchart LR
  Build[CALVER 빌드] --> Sign[Developer ID 서명]
  Sign --> Notarize[공증 제출·대기]
  Notarize --> Staple[staple]
  Staple --> Gatekeeper[spctl 검증]
  Gatekeeper --> Publish[수동 배포]
```

RC에서 clean macOS 계정으로 folder/file/CLI와 local-only 동작을 확인한 뒤 stable을 만든다. 자동 업데이트는 이번 범위에 없으므로 새 DMG를 수동 설치한다. 문제 발생 시 배포 파일을 내리고 직전 CALVER DMG를 다시 게시한다. 이미 생성한 사용자 review 데이터는 schema 호환 여부를 먼저 확인하며 무조건 삭제하지 않는다.
