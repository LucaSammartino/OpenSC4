| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lotrec-replace18: specified live lot getter comparison | Codex / `glue-lotrec-replace18` (engine lane) | ready for measured shadow work only; cargo 7531/7531, clippy clean, CTest 87/87; lotrec and lotrecview 73/73 exact through 2032 | Establish the supported developer/lot-manager replacement boundary before enabling lotrec replacement. |

Implemented in f7a6f8001 and 9cbb12158: default-off `lotrecview_shadow` compares center x/y/z, flags and altitude against the Rust record projection; complete-record validation precedes publication.
Tests cover invalid records/buffers, exact float bits, signed zero, NaN payloads, getter slots and the unavailable factory/default-off boundary.
This round recognizes `lotrecview_shadow` as an ini key and tests that opting in produces no unknown-key warning.

Final gate: `qbuild all` PASS; cargo 7531 passed, 0 failed; clippy 0 diagnostics; native build 48/48 edges; CTest 87/87.
Native verdict: `build-runs/build-results/qbuild-20261025-030453-build-ctest.verdict`.
Explicit Rust formatting check passed for both owned Rust files using absolute paths (initial relative-path invocation failed to locate them).

`lotrec18-r3-view-20261025a`: clean through 2032-01-01, ref_warnings=0; both shadows 73/73 exact; exposed the three configuration warnings fixed here.
`lotrec18-r3-view-20261025b`: final DLL 2c288964, clean through 2032-01-16, ref_warnings=0; panics, exceptions and other warnings all 0.
`lotrec18-r3-view-20261025b lotrec 73/73`; `lotrec18-r3-view-20261025b lotrecview 73/73`; every observation mismatches=0, no resyncs.
At 2032-01-01 both final observations were exact=665/665, differences=0, errors=0.
Requests, complete logs and verdicts remain under `build-runs/game-queue/done/` and `build-runs/game-results/`.

Missing: cSC4Lot replacement remains unavailable. Spec §15.1 explicitly excludes standalone complete-owner activation with native developers retained; §13 also leaves live signatures incomplete.
No replacement factory or full-house replacement run is claimed. Do not append lotrec to the house replace list on this evidence.
The ready entry covers the tested, opt-in getter comparison and factory-boundary tests. No public C ABI exports changed; no new questions or spec edits.
