# Contract: Child stream policy

## Protocol stream

- ACP stdout처럼 의미 있는 frame을 운반하는 stream이다.
- frame 최대 크기까지 exact bytes를 전달한다.
- 최대 크기 + 1 byte, malformed frame, frame 중간 EOF는 typed fatal failure다.
- 일부 bytes를 truncate/drop한 뒤 다음 frame을 정상 처리하지 않는다.
- fatal failure가 정해지면 stderr drain과 process tree cleanup은 끝까지 수행한다.
- bounded ingress가 가득 차면 protocol reader는 backpressure를 적용하되 status/cancel/shutdown control task를 막지 않는다.

## Parsed capture

- Git/curl/PATH probe처럼 완료된 stdout/stderr를 parser가 소비한다.
- 한도 안의 complete output만 성공이다.
- overflow, timeout, invalid required encoding은 typed failure이며 partial success가 아니다.

## Display log

- terminal output와 ACP stderr처럼 사용자/운영자에게 보여 주는 stream이다.
- byte/event/rate 한도를 넘는 부분은 정책대로 drop/truncate할 수 있다.
- `truncated`, dropped byte/event counters와 한 번의 marker를 제공한다.
- 줄바꿈 없는 입력도 event-byte 상한으로 잘라 drain을 계속한다.
- invalid UTF-8은 replacement 또는 bytes metadata로 표시하며 무제한 재시도를 하지 않는다.

## 동시성

- stdout/stderr는 별도 drain task/channel을 가진다.
- 한 stream의 느린 consumer가 다른 stream, wait, cancel을 막지 않는다.
- wait는 두 drain의 종료 또는 명시적 abort 결과를 합쳐 최종 outcome을 만든다.

## 회귀 matrix

| Case | Expected |
|---|---|
| protocol limit exactly | frame delivered |
| protocol limit + 1 | typed failure, tree terminated, next frame not delivered |
| malformed JSON-RPC | typed failure, pending request failed |
| mid-frame EOF | typed failure |
| stderr 100 MiB no newline | bounded log, cancel responsive |
| stdout+stderr pressure | no deadlock, both counters correct |
| capture overflow | no partial parsed success |
| secret sentinel | absent from diagnostics |
