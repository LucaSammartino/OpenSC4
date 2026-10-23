| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netmgr-exact5: network-lot record 11 shadow | Codex, `.claude/worktrees/glue-netmgr-exact5` (engine lane) | ready; cargo 6232/0/0, clippy clean, CTest 68/68; run 323 clean, netmgr 73/73, 0 resyncs; 3 C ABI additions; no questions | 1. Coordinator review and coverage credit. |

## Report

`netmgr-exact5-r2-2032 73/73`: one load line and 72 month lines, every mismatch count zero, zero configuration differences and zero resynced lines. Konradshohe reached 2032-01-03; verdict clean, ref_warnings=0, foreign exceptions=0, stalls=0, rtc=0. Run record: [323](../runs/netmgr-exact5-r2-2032-323.md).

The original April record 11 gap had two causes. The core's lot-state boundary walk omitted the north columns specified by network-manager §6.3; the pending set also needed the SGI bucket/rehash processing order in §8.5 NM13. Commit `7812f988` implemented these, captured the generator input per §17.3, compared configuration choices, and removed record resyncs and the NM10/NM11 suppression. Diagnostic corrections and updated conformance expectations are in `cb48b856`.

Strict replay then exposed a shadow sampling bug: every configuration was registered with token 0 and an empty object array, and the read-only host supplied no object-ID callback. Mandatory prop/flora random draws were missing. In the first strict run March's next-word positions diverged 322/318 and April's 327/319; subsequent configuration choices diverged. April lot counts already agreed, 527/527. That run was clean but only `netmgr-exact5-r1-2032 2/73`, with zero resynced lines; [run 322](../runs/netmgr-exact5-r1-2032-322.md) preserves the failed comparison.

Commit `0baa6a1f` completes the shadow input: decode kept objects through the existing Rust `fc4_lot_config_object_array` at detail 3 (prop-lotconfig §B3.3), assign stable nonzero configuration tokens, retain uniquely owned SDK object records, and expand families through the city's public `GetObjectIDArray` interface. SDK vector payloads are freed during city shutdown. No guessed native signatures or C++ simulation rules were added. The focused test verifies configuration/mandatory-prop/flora draws across an RNG reload, including a one-ID prop, optional props and empty lists.

Final April line: mismatches=0, lots=527/527, zone diffs=0, wealth diffs=0, configs same=527, draws=0, incompatible=0. Record 11 has the same decoded entries; its serialized map iteration order can differ (`same entries (order differs)`). The existing comparator compares entries by key; 73/73 is that semantic comparison, not a claim of identical serialized byte order. No new exclusions or resyncs were introduced.

Validation on final engine source `0baa6a1f`:

- Focused object tests: 2 passed, 0 failed. Owned Rust formatting: PASS.
- Required `qbuild all`: workspace tests 6232 passed, 0 failed, 0 ignored; clippy exit 0 with zero diagnostics.
- Native queue `qbuild-20261023-204924-build-ctest`: build PASS 50/50 edges, CTest PASS 68/68 (152.7 s), qbuild PASS. Session 92935 collected to terminal exit 0. Later commits only record results.
- DLL SHA256: `29012747892A86BDED77225B6113002D1AA3DDB99A44B3894BD1989CC75DBD58`.
- Turbo request `netmgr-exact5-r2-2032`: queued 21:02:33, started 21:02:35 in slot 1, clean at 21:03:48; 58.578 simulation seconds, 81 s wall. Wait session 88372 collected to exit 0. No outstanding jobs or game requests.

Evidence stays in `build-runs/`: `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`, `build-results/qbuild-20261023-204924-build-ctest.verdict`, and `game-results/netmgr-exact5-r{1,2}-2032.{log,verdict}`. Prior diagnostic failures remain in the `*-failed-r1.log` files; the initial gate failed on an old conformance event order and clippy `int_plus_one`, both corrected before the final passing gate.

No turbo-only netmgr discrepancy remains, so no normal-speed rerun was needed. There are no netmgr resyncs to correlate with train152's one-off traffic day-line difference; this run requested only the netmgr shadow. Shadows remain default-off and are enabled here only by `netmgr_shadow=1`.

Questions: none. C ABI additions from the first round: `fc4_netlot_set_random_state`, `fc4_netlot_get_random_state`, `fc4_netlot_pending_buckets`; no ABI version bump. No edits to generated ABI entries, coverage maps or tools/regress.ps1. Coordinator: review the run and credit coverage; verify the shared regress profile as needed. The supervisor publishes these detached-HEAD commits to the reserved branch.
