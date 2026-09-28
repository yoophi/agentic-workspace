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
REALPY=$(command -v python3)
cat > "$T/mockbin/python3" <<MOCK
#!/bin/bash
# 모의 명령: 스모크 도우미 스크립트만 가짜로, 인라인 python(-c)은 실제 python으로.
case "\$1" in
  -c) exec "$REALPY" "\$@" ;;
  *owner-check.py) echo '{"mock":true}'; exit "\${MOCK_OC:-0}" ;;
  *bench-check.py) echo "{\"runListed\": \${MOCK_RUN_LISTED:-false}}" ;;
  *token-check.py) [ "\${MOCK_TOKEN:-401}" = error ] && exit 1; echo "{\"status\": \${MOCK_TOKEN:-401}}" ;;
  *) echo '{"busyRuns": 0}' ;;
esac
MOCK
chmod +x "$T/mockbin/python3"
tail_run() { # <SENT> <MOCK_OC> [<QUIT> <TOKEN 1|''> <MOCK_TOKEN>] — 앱 pid는 이미 사라진 D(소멸), 서버 신호 없음
  ( R="$T/tail-$1-$2-${3:-g}-${4:-x}-${5:-x}"; mkdir -p "$R"; : > "$R/meta.txt"; log() { echo "$*" | tee -a "$R/meta.txt"; }
    APID=$D; SPID=$D; SCMD=none; SRV_EXE=/nonexistent; DATA="$T/data"; RUN=r1; QUIT=${3:-g}; SENT=$1; TOKEN=${4:-}; BUSY=
    SERVER_LINES_BEFORE=0; export MOCK_OC=$2 MOCK_TOKEN=${5:-401}; PATH="$T/mockbin:$PATH"
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
# 6) quit-run TOKEN=1(Codex r9 docs): 정상 종료(c)는 401이어야, SIGTERM(g) 대조는 200이 기대값. 어긋나거나 검사 오류면 8.
out=$(tail_run yes 0 c 1 401); rc=$?; check "quit c token 401: exit code" "$rc" 0
out=$(tail_run yes 0 c 1 200); rc=$?; check "quit c token still 200: exit code" "$rc" 8
check "quit c token still 200: result line" "$(echo "$out" | grep -c '^smoke-result=token-check-failed')" 1
out=$(tail_run yes 0 c 1 error); rc=$?; check "quit c token check error: exit code" "$rc" 8
out=$(tail_run yes 0 g 1 200); rc=$?; check "quit g token 200 (contrast): exit code" "$rc" 0
out=$(tail_run yes 0 g 1 401); rc=$?; check "quit g token 401 unexpected: exit code" "$rc" 8
out=$(tail_run no 0 c 1 200); rc=$?; check "invalid path wins over token: exit code" "$rc" 5
# 7) close-run.sh의 판정 구간(`ok=no`부터 끝)을 그대로 떼어 모의 입력으로 돌린다: run 잔존 7, 토큰 200·검사 오류 8, 통과 0.
CR="$(dirname "$0")/close-run.sh"; CTAIL="$T/close-run-tail.sh"; sed -n '/^ok=no$/,$p' "$CR" > "$CTAIL"
check "close tail extracted" "$(head -1 "$CTAIL")" "ok=no"
close_run() { # <MOCK_RUN_LISTED true|false> <SCEN quit|close-token> <MOCK_TOKEN>
  ( R="$T/close-$1-$2-$3"; mkdir -p "$R"; : > "$R/meta.txt"; : > "$R/app.log"; log() { echo "$*" | tee -a "$R/meta.txt"; }
    APID=$D; SPID=$D; DATA="$T/data"; RUN=r1; SCEN=$2; WINRC=0; export MOCK_RUN_LISTED=$1 MOCK_TOKEN=$3; PATH="$T/mockbin:$PATH"
    . "$CTAIL" )
}
out=$(close_run false close-token 401); rc=$?; check "close ok: exit code" "$rc" 0; check "close ok: result" "$(echo "$out" | grep -c '^close-result=ok')" 1
out=$(close_run false close-token 200); rc=$?; check "close token not revoked: exit code" "$rc" 8
out=$(close_run false close-token error); rc=$?; check "close token check error: exit code" "$rc" 8
out=$(close_run true quit 401); rc=$?; check "close run left: exit code" "$rc" 7
check "close run left: result" "$(echo "$out" | grep -c '^close-result=run-not-removed')" 1
out=$(close_run false quit 200); rc=$?; check "close without token scenario: exit code" "$rc" 0
check "close-run exits with the final result" "$(grep -c '^exit "$FINAL"$' "$CR")" 1
# 8) close-run.sh의 닫기 동작부터 끝까지(`windows-before-close` 줄부터)를 그대로 떼어 모의 입력으로 돌린다(Codex r10 docs):
#    동작 실패 6, 예상 밖의 전체 종료·창 남음 9. sx·send_key_to는 이 하위 셸에서만 모의(MOCK_SX_RC·MOCK_WINDOWS).
ATAIL="$T/close-run-action.sh"; sed -n '/^log "windows-before-close=/,$p' "$CR" > "$ATAIL"
check "close action tail extracted" "$(head -1 "$ATAIL" | grep -c '^log "windows-before-close=')" 1
sleep 300 & LIVE=$!
close_action() { # <path> <MOCK_SX_RC> <alive yes|no> <MOCK_WINDOWS>
  ( R="$T/action-$1-$2-$3-${4// /_}"; mkdir -p "$R"; : > "$R/meta.txt"; : > "$R/app.log"; log() { echo "$*" | tee -a "$R/meta.txt"; }
    sx() { case "$1" in *"every window"*) echo "$MOCK_WINDOWS" ;; *) echo "mock"; return "$MOCK_SX_RC" ;; esac; }
    send_key_to() { return 0; }; invalid_stop() { echo "invalid-stop: $1"; exit 6; }
    [ "$3" = yes ] && APID=$LIVE || APID=$D; SPID=$D; DATA="$T/data"; RUN=r1; SCEN=quit; CLOSE=$1; MAIN="Agentic Workbench"; BID=x
    export MOCK_SX_RC=$2 MOCK_WINDOWS=$4 MOCK_RUN_LISTED=false; PATH="$T/mockbin:$PATH"
    . "$ATAIL" )
}
out=$(close_action a 0 yes Settings); rc=$?; check "close a ok: exit code" "$rc" 0
out=$(close_action a 1 yes "Settings, Agentic Workbench"); rc=$?; check "close a click failed: exit code" "$rc" 6
check "close a click failed: result" "$(echo "$out" | grep -c '^window-result=action-failed')" 1
out=$(close_action a 0 no ""); rc=$?; check "close a whole app exited: exit code" "$rc" 9
out=$(close_action b1 0 yes "Settings, Agentic Workbench"); rc=$?; check "close b1 target window left: exit code" "$rc" 9
out=$(close_action b1 0 yes Settings); rc=$?; check "close b1 ok: exit code" "$rc" 0
out=$(close_action b2 0 no ""); rc=$?; check "close b2 whole app exited (known b2 risk): exit code" "$rc" 9
out=$(close_action f 0 no ""); rc=$?; check "close f ok: exit code" "$rc" 0
out=$(close_action f 1 no ""); rc=$?; check "close f click failed: exit code" "$rc" 6
out=$(close_action f 0 yes Settings); rc=$?; check "close f app still running: exit code" "$rc" 9
check "close-run passes the window verdict to the final result" "$(grep -c 'close_final "$ok" "$TOKRC" "$WINRC"' "$CR")" 1
check "close-run keeps the click exit code" "$(grep -c 'ACTRC=\$?' "$CR")" 2
kill "$LIVE" 2>/dev/null
# 정리: 이 시험이 띄운 B·C만(신원을 바르게 다시 기록한 뒤) 끝낸다.
remember "$T" "$B" "$C"; kill_exact "$T/kills.txt" "$B" "$C"; sleep 0.3
check "cleanup" "$( (kill -0 $B 2>/dev/null || kill -0 $C 2>/dev/null) && echo alive || echo gone)" gone
cat "$T/kills.txt"; exit $fail
