# 042 구현 리뷰

## 1. OCR delegate-review (`--from 2e7f359 --to HEAD`, 46개 검토 대상 중 운영 코드 중심)

| # | 등급 | 지적 | 조치 |
|---|---|---|---|
| O1 | Medium | 접근 기록이 클라이언트가 보낸 `requestId`·`operation`(인증 전 요청 포함)을 그대로 찍는다. 식별자는 길이만 검사하므로 개행·공백으로 기록 줄이나 필드(`status=200`)를 위조할 수 있다 | `AccessEntry::line`이 두 값을 따옴표로 감싸 escape. 단위 `client_supplied_values_cannot_forge_lines_or_fields` |
| O2 | Medium | debug probe·진단 파일: 이미 있던 파일을 덮어쓰면 `mode(0o600)`가 적용되지 않아 기존 권한이 남는다(토큰 든 진단 파일) | 쓴 뒤 `set_permissions(0o600)` |
| O3 | Medium | MCP `requestId`를 trim해 키를 만들어 도메인에서 다른 `" r1"`·`"r1"`이 같은 키로 합쳐질 수 있다 | 원래 값으로 키, 공백뿐인 값만 제외. 단위 `request_ids_are_compared_verbatim` |
| O4 | 확인 | 열린 WebSocket이 graceful shutdown을 붙잡아 종료가 멈출 수 있는가 | 붙잡지 않음 — 회귀 시험 `http_tickets.rs` `shutdown_completes_with_open_subscriptions` 추가(통과) |

확인만 하고 고치지 않은 것: `drain::accept`의 증가 뒤 재확인은 close와 경합해도 drain 뒤에 작업이 실행되지 않는다(두 번째 확인이 거절). `RunEvent::Exit`의 `block_on`은 메인 스레드를 막지만 drain은 tokio 작업자에서 돌고, 데스크톱 bridge의 `window.eval`은 전달만 하고 기다리지 않는다(루프 종료 중이면 오류를 버린다).
