| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-renderprops-shadow13 (render properties shadow) | `glue-renderprops-shadow13` / engine lane | in progress | (1) Inspect existing shadow and contracts; (2) complete and gate; (3) measure city through 2032. |

## Handoff
Goal: default-off renderprops_shadow, compare native render-properties getters with core at load/monthly through 2032.
Worktree reserved by coordinator; detached HEAD; initial tree clean and main.. empty.
Existing default-off shadow already measures registry/owned fixture; RP7 explicitly forbids late history bootstrap and assumes no available trace/view bridge.
5001139a2: shared read-only registry comparator and nine owned-instance registry samples per monthly fixture (4,878 added fields): fresh A/B, A/B after each setter pass, B after A termination.
No ABI changes, no new ambiguous contract, no new questions; RP8 remains open for existing private fixture ABI.
Full qbuild all reached terminal PASS (session 43351 complete): Cargo 7,421/0/0 over 51 summaries; clippy diagnostics=0; queued native build PASS 264/264 edges; CTest PASS 81/81.
Native queue qbuild-20261024-204415-build-ctest completed via the required coordinator path. No duplicate verify needed. Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
Outstanding game request: grp13-2032-20261024-r1 (Konradshohe turbo to 2032-01-01; renderprops_shadow=1; seconds=900).
Foreground waiter session 65544 is running via build-runs/wait-game.py (brief prohibits qwait); collect this operation to terminal, then record exact/total and ref_warnings.
Highest local recorded run number observed: 632. No other request outstanding.
Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-renderprops-shadow13-r1-manifest.json.
