| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| netocc-cities30: census network shadows (engine) | Codex; .claude/worktrees/netocc-cities30; detached HEAD, reserved branch netocc-cities30 | continue; code 63ecff119, 93da4b299, 0069bdb53, baf23e1d1; ABI unchanged; Rust 7967, clippy 0, CTest 131/131; final caller-scope DLL not game-tested | 1. Run final DLL on North Weburg and Port with documented stall grace. 2. Run empty and remaining census scenarios. 3. Enqueue only after clean/exact qualification. |

- Acceptance: all census cities clean/exact is NOT met; no ready enqueue, coverage edits or new questions. Existing NM18-A/NM19-A runtime observations remain relevant.
- Startup merges requested: netocc-cities29 and main both already ancestors of initial HEAD ef84805f1; Git reported already up to date (also read-only packed-refs lock errors). No shared refs changed.
- NO46 implemented: prebuilt/bridge v4 + base v8; v3 discarded words; opaque byte; raw-u32 optional gate; tunnel v2 and segment pair; independent expected-byte normalization.
- NO47 implemented: empty successful captures compare equal and explicitly unexercised; absent classes receive no exercised-owner denominator. Empty census rerun remains pending.
- NM19 implemented: insertion re-reads forced state after object callbacks; direct setter/redevelopment observations feed replay; in-flight generator capture is refused; no generator resync.
- Tests: raw-minus-zero gate, v3 canonicalization, tunnel reference, truncation/version failures, forced/locked save/load and callback changes; direct-seed versus old-neighbour worked case; provenance negative cases and ABI caller-range boundaries.
- Final frozen-source gate PASS: Rust 7967/0, clippy 0 diagnostics, CTest 131/131; logs build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log. Owned Rust fmt 5 files/0 differences. Gate rerun only after runtime fixes.
- All game logs below: C:/Users/tlori/FreeCity4/build-runs/game-results/<label>.log and .verdict; all ref_warnings=0. Requests used the prior lane's unchanged scripts via the shared main queue.
- Earlier DLL e17fb947: netocc-cities30-a-tegel clean/end date; netocc 2/2, netmgr 2/3, 23 old-neighbour differences. Native ordered seeds equal; removals and replacement insertions captured.
- DLL 6c04c4ec (before final caller-scope fix): netocc-cities30-b-tegel clean/watchdog0/end date; netocc 2/2, netmgr 3/3 exact; 144 native seeds, 204 removals, 60 insertions. Ordered same-call data4=1 replacement evidence replaces adjacency inference.
- Same DLL: netocc-cities30-c-bevopolis stalled at 2257-04-21/watchdog1; load netocc 1/1 (654 prebuilt records), netmgr 1/1; later samples missing. Partial evidence only.
- Same DLL: netocc-cities30-d-northweburg clean-labelled/end date BUT watchdog1; netocc 3/3 including tunnels, netmgr 3/4. One pass reports 230 differences: direct-group mismatch plus 229 pipe removals, not a qualified pass.
- North Weburg trace: msg 278E6003 data 00040036/00040036/0/1; native slot-50 groups contain 563 surface and 229 pipe seeds; core direct group has 563. network-tools section 11 documents destruction nested inside underground refitting.
- Final baf23e1d1 fix observes manager slot-6 caller against published ABI extents, separating terrain-handler seeds from underground-refit seeds; requires witnessed refit seed membership/removal, preserves unknown callers. This final fix passed the gate but has NO game verdict yet.
- Same older DLL: netocc-cities30-e-port stalled at 2040-12-12/watchdog1, before NM19-A's January event; load netocc 1/1 and netmgr 1/1 only; no direct setter/redevelopment observed before termination.
- Runtime next: engine/README.md lines 741-746 documents autotest_stall_grace_ms up to 120000; prior requests omitted it. Busy native actions exceeded the ordinary 5/10-second watchdog; preserve failed controls and retry with this documented allowance, watchdog/hard deadline active and comparisons unchanged.
- Next: final-DLL Tegel/North/Bevopolis/Port reruns, lua-lots-empty (report unexercised separately), then the full prior 12-city broad qualification. Verify all counters, end dates and actions; do not credit failed/partial runs or absent owners.
- Guard at code commit baf23e1d1: 195706/200000 measured context. Continue required; no merge-queue file written.
