| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-fix5: isolate netocc shadow | Codex / glue-netocc-fix5 | in progress | Resolve requested merge, reproduce CTest failures, fix and verify city through 2032. |

## Handoff
- Goal: merge glue-netocc-shadow5 and repair plugin exit/write-count regressions; retain exact 73/73 city shadow.
- Merged requested branch in fd035d87; resolved config/observer conflicts additively.
- Original merged source qbuild verify PASS: build 221/221, CTest 69/69 (193.5 s), request qbuild-20261023-230531-build-ctest.
- Train157 failures did not reproduce in this fresh build; do not claim a confirmed source-level crash cause.
- Root cause confirmed: Ninja deps for cached FC4NetMgrShadow.cpp.obj point to ../../../train-integration/engine/plugin/FC4WriteCapture.h, so local layout changes did not rebuild its static captures.
- Debugger catches AV in WriteCapture destructor for FC4NetMgrShadow's g_ourLot; facts in build-runs/exit-diagnostic.txt, debugger script debug-native-exit.ps1 (only our test/DLL inspected).
- Experimental subclass reverted; preserve original branch variant implementation. CMake now adds explicit local capture-header dependencies to plugin C++ sources to prevent stale layouts.
- Netocc requires its ini key; absent/off key stores no city pointer and never captures, including when the old environment variable is set.
- Isolation committed as 7ca6b5fe; qbuild all completed FAIL: native build 76/76, CTest 67/69 (127.7 s).
- Final workspace tests PASS 6653/0/0; clippy PASS, zero diagnostics.
- Failed native request qbuild-20261023-231436-build-ctest: plugin_disable_test and plugin_ini_dialogs_test exit AV; other original regressions now pass. Evidence build-runs/failed-isolation1-ctest.log.
- Focused hung operation 72265 terminated/collected (Ctrl-C, exit 1); debugger tests reached terminal C0000005. No diagnostic operation remains outstanding.
- Dependency fix committed as 21b0c98b7; corrected-source qbuild all collected to terminal PASS (shell operation 47442 complete).
- Corrected-source cargo 6653/0/0 and clippy zero diagnostics PASS; native qbuild-20261023-233345-build-ctest PASS build 203/203 (8.4 s), CTest 69/69 (79.2 s).
- Generated Ninja edge explicitly depends on this worktree's WriteCapture.h; NetMgrShadow rebuilt at edge 129/203. DLL SHA256 67b7a633b592055a2e2509c0ef39485a636e95597c5ee5610b88d08dcedaa017.
- City request netocc-fix5-2032-21b0c98b7-1 outstanding: Konradshohe, turbo=1, end_date=2032-01-01, netocc_shadow=1. No other game request.
- Blocking filesystem-watcher operation shell session 93181 is waiting; collect it, no qwait and no duplicate request.
- Next: inspect verdict and run build-runs/summarize-netocc-fix5.py with label; record run and final report, commit, enqueue ready if exact.
- Detached HEAD is intentional; never switch or update refs.
