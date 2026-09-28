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
# 4) 최종 판정·종료 코드(Codex r7 docs): quit-run.sh (g)와 같은 문장(send_sigterm → SENT, quit_verdict → VERDICT, exit)을 하위
#    셸에서 그대로 돌려, 출력과 하위 셸 종료 코드를 모두 본다.
g_run() { # <pid> — 보낸 뒤 quit-run.sh와 같은 방식으로 PID 소멸을 관측한다(상한 30초 조건 대기).
  ( SENT=no; if send_sigterm "$T/kills.txt" "$1"; then SENT=yes; else echo "sigterm-not-sent: the target identity is gone or changed (attempt invalid)"; fi
    gone=no; for i in $(seq 1 60); do kill -0 "$1" 2>/dev/null || { gone=yes; break; }; sleep 0.5; done
    quit_verdict "$SENT" "$gone" g; VERDICT=$?; exit "$VERDICT" )
}
#   4a) 대상이 신호 전에 사라짐(D): 미전송 + PID 소멸 → path-exercised=no, 종료 코드 5
out=$(g_run "$D"); rc=$?
check "gone before send: exit code" "$rc" 5
check "gone before send: verdict" "$(echo "$out" | grep -c '^path-exercised=no')" 1
check "gone before send: never yes" "$(echo "$out" | grep -c '^path-exercised=yes')" 0
#   4b) 신원 일치(E): 전송 + PID 소멸 → path-exercised=yes, 종료 코드 0
sleep 300 & E=$!; remember "$T" "$E"; out=$(g_run "$E"); rc=$?
check "sent and gone: exit code" "$rc" 0; check "sent and gone: verdict" "$(echo "$out" | grep -c '^path-exercised=yes')" 1
#   4c) 전송했지만 앱이 남음 → path-exercised=no, 5
out=$(quit_verdict yes no g); rc=$?
check "sent but still running: exit code" "$rc" 5; check "sent but still running: verdict" "$(echo "$out" | grep -c '^path-exercised=no')" 1
#   4d) quit-run.sh가 실제로 이 함수들과 최종 exit를 쓰는지(구조 확인)
QR="$(dirname "$0")/quit-run.sh"
check "quit-run uses send_sigterm for g" "$(grep -c 'if send_sigterm "$R/kills.txt" "$APID"; then SENT=yes' "$QR")" 1
check "quit-run judges with quit_verdict" "$(grep -c 'quit_verdict "$SENT" "$gone" "$QUIT"' "$QR")" 1
check "quit-run combines verdict and owner-check" "$(grep -c 'smoke_final "$VERDICT" "$OC"' "$QR")" 1
check "quit-run keeps the owner-check exit code" "$(grep -c '^OC=\$?$' "$QR")" 1
check "quit-run exits with the final result" "$(grep -c '^exit "$FINAL"$' "$QR")" 1
check "quit-run has no unconditional yes" "$(grep -c 'log "path-exercised=yes"' "$QR")" 0
# 5) 실제 quit-run.sh의 판정 이후 구간(`gone=no`부터 끝까지)을 그대로 떼어 모의 입력으로 돌린다(Codex r8 docs): owner-check
#    실패가 최종 종료 코드에 반영되는지. python3은 이 하위 셸에서만 모의 명령으로 바꾼다(owner-check는 MOCK_OC로 끝남).
TAIL="$T/quit-run-tail.sh"; sed -n '/^gone=no$/,$p' "$QR" > "$TAIL"
check "tail extracted from quit-run" "$(head -1 "$TAIL")" "gone=no"
mkdir -p "$T/mockbin" "$T/data/workbench/server"; : > "$T/data/workbench/server/server.log"
cat > "$T/mockbin/python3" <<'MOCK'
#!/bin/bash
case "$1" in *owner-check.py) echo '{"mock":true}'; exit "${MOCK_OC:-0}" ;; *) echo '{"busyRuns": 0}' ;; esac
MOCK
chmod +x "$T/mockbin/python3"
tail_run() { # <SENT> <MOCK_OC> — 앱 pid는 이미 사라진 D(소멸), 서버 명령줄은 불일치라 서버 신호 없음
  ( R="$T/tail-$1-$2"; mkdir -p "$R"; : > "$R/meta.txt"; log() { echo "$*" | tee -a "$R/meta.txt"; }
    APID=$D; SPID=$D; SCMD=none; SRV_EXE=/nonexistent; DATA="$T/data"; RUN=r1; QUIT=g; SENT=$1; TOKEN=; BUSY=
    SERVER_LINES_BEFORE=0; export MOCK_OC=$2; PATH="$T/mockbin:$PATH"
    . "$TAIL" )
}
out=$(tail_run yes 1); rc=$?
check "valid path + owner-check failure: exit code" "$rc" 6
check "valid path + owner-check failure: result line" "$(echo "$out" | grep -c '^smoke-result=owner-check-failed')" 1
check "valid path + owner-check failure: path still exercised" "$(echo "$out" | grep -c '^path-exercised=yes')" 1
out=$(tail_run yes 0); rc=$?
check "valid path + owner-check ok: exit code" "$rc" 0; check "valid path + owner-check ok: result line" "$(echo "$out" | grep -c '^smoke-result=ok')" 1
out=$(tail_run no 0); rc=$?
check "invalid path + owner-check ok: exit code" "$rc" 5
out=$(tail_run no 1); rc=$?
check "invalid path + owner-check failure: exit code stays 5" "$rc" 5
# 정리: 이 시험이 띄운 B·C만(신원을 바르게 다시 기록한 뒤) 끝낸다.
remember "$T" "$B" "$C"; kill_exact "$T/kills.txt" "$B" "$C"; sleep 0.3
check "cleanup" "$( (kill -0 $B 2>/dev/null || kill -0 $C 2>/dev/null) && echo alive || echo gone)" gone
cat "$T/kills.txt"; exit $fail
