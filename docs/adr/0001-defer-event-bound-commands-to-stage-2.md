---
status: accepted
date: 2026-09-26
---

# 이벤트·창 정체에 묶인 Tauri command 32개는 2단계에서 옮긴다

서버-클라이언트 전환 1단계(037·038)는 AW의 Tauri command를 `Workbench` 인터페이스 뒤로 옮기지만, agent run 8·agent exchange 4·orchestration 18·worktree watcher 2개는 `window.label()`로 소유자와 이벤트 대상을 정하고 Tauri 이벤트 sink·메모리 journal·MCP 서버 상태에 묶여 있다. 이 32개는 1단계에서 옮기지 않고, 이벤트 봉투와 `window_label` 분해를 다루는 정본 2단계에서 함께 옮긴다. 지금 옮기면 계약(`CallRequest`/principal)에 데스크톱 창 정체를 임시 필드로 심고 2단계에서 다시 뜯어내야 하기 때문이다.

## Considered Options

- 61개 전부 이관(표현 상태 8개만 제외), 임시 `clientInstanceId` 필드 도입 — 정본 1단계 문구는 그대로 달성하지만 버릴 계약이 생기고 PR이 037의 3배 이상이 된다.
- 038a(29개) → 038b(32개, 임시 창 정체) 두 PR — 리뷰 단위는 작지만 같은 이중 작업이 남는다.

## Consequences

- 1단계 완료 조건 "Tauri command가 `Workbench`만 호출한다"는 "옮길 수 있는 전부"로 재정의된다. 정확한 71개 분류는 [`docs/workbench-seam.md`](../workbench-seam.md)의 inventory 표가 정본이다.
- 2단계(039)는 32개 command 이관과 이벤트 모델 통합을 함께 다루므로 037·038보다 크다.
- 데스크톱 표현 상태 8개(글꼴 3·panel layout 2·창 열기 2·외부 URL 1)는 이연이 아니라 영구 유지다(정본 배치표).
