# Context Map

이 저장소는 여러 컨텍스트로 나뉜다. 컨텍스트 문서는 용어가 실제로 정리될 때 만든다(`docs/agents/domain.md`).

## Contexts

- [Workbench](./crates/workbench-core/CONTEXT.md) — AW가 서버로서 소유하는 상태(프로젝트·프롬프트·목표·설정)와 사용자 저장소·파일 조회, 그리고 모든 클라이언트가 같은 계약으로 부르는 operation

## Relationships

- **AW 데스크톱 셸 → Workbench**: 데스크톱 command는 입력·결과 변환만 하고 Workbench operation을 호출한다. 창·글꼴·layout 같은 데스크톱 표현 상태는 셸이 소유하며 Workbench에 없다.
- **Workbench → git-core / acp-agent-core**: Workbench는 두 공유 crate의 모델(Git 변경·이력, agent catalog)을 그대로 사용하고 자기 용어로 다시 정의하지 않는다.
