| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-occmgr4 (verify the occupant-manager shadow, native original + occmgr_shadow=1) | `verify-occmgr4` / (engine lane) | in progress (round 2): call-follow mode occmgr_shadow_follow=1 | see Handoff |

## Handoff
Goal (round 2): followed-core query months (round 1: only 49/72, 158/188 on fresh cores) and real Move coverage,
by following the original's CALLS (slots 20-24 hooked, args captured before, core call after) instead of messages.
Round 1 (kept below in git history 6a1b4d37): 72/72 turbo, 188/188 normal exact, resyncs 0, ref 0.
Done: merged main47 (dfac0da6, FC4Config conflict resolved additively); call follow in FC4OccMgrShadow.cpp
(occmgr_shadow_follow=1, default 0 = round-1 message mode unchanged); key registered in FC4Config; quick build PASS.
Next: game run turbo label vom4-calls-2032a (occmgr_shadow_follow=1), then normal vom4-calls-normal-a; full gate.
Facts: month line now adds "calls ins/rem/remall/move(post) returns=a/b unfollowed nested; sync moves; pending".
Sync Move (delivered inside the original Move) compared on its own stream; queued streams compared by prefix,
tails carried to next month. Comparison selection (set-aside -> fresh core) unchanged.
