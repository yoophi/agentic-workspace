#!/bin/bash
# 모의 시험(Codex r6 docs): 신호는 기록한 신원(시작 시각·명령줄)이 같을 때만 간다. 이 시험이 띄운 `sleep`에만 신호를 보낸다.
# 사용: selftest-signal.sh <새 결과 디렉터리>
. "$(dirname "$0")/lib.sh"
T=${1:-}; need "$T" dir; [ -e "$T" ] && { echo "exists: $T"; exit 8; }; mkdir -p "$T"
fail=0; check(){ if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1 (got $2, want $3)"; fail=1; fi; }
# 1) 신원 일치 → 신호, rc 0
sleep 300 & A=$!; remember "$T" "$A"; kill_exact "$T/kills.txt" "$A"; rc=$?; sleep 0.3
check "matching identity: rc" "$rc" 0; check "matching identity: signalled" "$(kill -0 $A 2>/dev/null && echo alive || echo gone)" gone
# 2) 명령줄 불일치(다른 프로세스가 같은 pid를 쓴 것처럼) → 신호 없음, rc 1
sleep 300 & B=$!; echo "Mon Jan  1 00:00:00 2001     sleep 300" > "$T/pids/$B"; kill_exact "$T/kills.txt" "$B"; rc=$?
check "changed start time: rc" "$rc" 1; check "changed start time: not signalled" "$(kill -0 $B 2>/dev/null && echo alive || echo gone)" alive
sleep 300 & C=$!; echo "$(ps -o lstart= -p $C)     /usr/bin/other-command" > "$T/pids/$C"; kill_exact "$T/kills.txt" "$C"; rc=$?
check "changed command line: rc" "$rc" 1; check "changed command line: not signalled" "$(kill -0 $C 2>/dev/null && echo alive || echo gone)" alive
# 3) 대상 소멸 → 신호 없음, rc 1
sleep 300 & D=$!; remember "$T" "$D"; kill "$D"; wait "$D" 2>/dev/null; kill_exact "$T/kills.txt" "$D"; rc=$?
check "gone target: rc" "$rc" 1
# quit-run (g) 분기와 같은 판정: rc가 0이 아니면 무효로 기록
if ! kill_exact "$T/kills.txt" "$D"; then echo "sigterm-not-sent: the target identity is gone or changed (attempt invalid)" > "$T/g-branch.txt"; fi
check "g branch marks invalid" "$(grep -c 'attempt invalid' "$T/g-branch.txt")" 1
# 정리: 이 시험이 띄운 B·C만(신원을 바르게 다시 기록한 뒤) 끝낸다.
remember "$T" "$B" "$C"; kill_exact "$T/kills.txt" "$B" "$C"; sleep 0.3
check "cleanup" "$( (kill -0 $B 2>/dev/null || kill -0 $C 2>/dev/null) && echo alive || echo gone)" gone
cat "$T/kills.txt"; exit $fail
