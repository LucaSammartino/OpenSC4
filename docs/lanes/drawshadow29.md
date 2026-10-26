| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| drawshadow29: 2D draw-stream oracle (engine) | Codex / drawshadow29 / detached HEAD; reserved branch drawshadow29 | blocked; code 50cad807e; ABI exports unchanged; full gate uncompleted; pilot complete=0 | 1. Resolve DS29-1. 2. Bind MapView draw/message replay and all outputs. 3. Complete full gate and game verification before enqueue. |

- Pilot: cSC4WinMapView; largest reached implemented candidate, and gen27 actually enters Update.
- House reach: 7/30 MapView methods, 5,574 bytes, Update=0; build-runs/drawshadow29-house-r2a.reach.tsv.
- Gen27 reach: 13/30 MapView methods, 15,665 bytes, Update=24, DoMessage=178/32; build-runs/drawshadow29-gen27-r2a.reach.tsv.
- Other candidates: SubBudgetWindow, TreeView, ListBox each 0 reached methods/bytes in both traces.
- Prior game results: main build-runs/game-results/drawshadow29-house-r2a.verdict clean, ref_warnings=0; drawshadow29-gen27-r2a.verdict corrupt, foreign_exceptions=1, ref_warnings=0. Reach evidence only; no pilot comparison credit.
- Reusable module: FC4DrawStream.h and FC4DrawStreamCapture.*; recording context now replays native getter snapshots without drawing or calling borrowed objects again.
- Replay compares independently supplied call arguments, copies rectangles/origins, preserves borrowed engine identity and explicit image TGI keys, and refuses unsupported/malformed calls; query storage is bounded.
- Default-off drawstream_shadow key registered; capture installation is guarded against repetition; observer serialization cannot repeat native paint or swallow its exception.
- Focused validation: qbuild quick PASS, build 53/53 edges; engine/build-x86/fc4_draw_stream_test.exe exit 0 (snapshot mutation, replay, identity, keyed blit, malformed/extra calls, bounds).
- Full gate: attempted once on 50cad807e; interrupted after >1,823 s waiting for three occupied cargo slots; no gate verdict. See build-runs/qbuild-all-r3-status.txt; older gate logs are not current evidence.
- Pilot Draw/Update and message verification: 0 compared, 0 complete; no method-credit lines or whole-row credit claimed. Font/surface/owner/message adapters remain unbound; widget traces remain explicitly incomplete.
- Blocker: DS29-1 appended to docs/specs/questions/map-view.md; full live-owner snapshot offsets/types and bitmap presentation boundary are missing from the permitted interface sources. SDK/curated interfaces expose no alternative MapView snapshot API.
- No game run on this round's final source; the new startup diagnostic is not in the earlier quick-built DLL. No ready enqueue.
- Next: obtain the missing ABI binding; connect Rust MapView execution to the same borrowed query inputs and recording surface/context, compare all owner outputs and recorded messages/commands, then finish full gate and a clean full game run with exact complete method lines.
