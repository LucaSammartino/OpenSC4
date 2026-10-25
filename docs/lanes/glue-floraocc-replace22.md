| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-floraocc-replace22: Rust flora occupant persistence | `glue-floraocc-replace22` (engine; detached HEAD) | Ready for measured persistence work: cargo 7589 passed, clippy 0 diagnostics, CTest 90/90; both game runs clean through 2032 with ref_warnings=0 | Complete runtime occupant/static-data replacement and plugin registration/ownership proof |

Rust owns version-3 flora occupant records, including base property overrides, position, dates, state, orientation and lot tag, through additive C ABI ownership/persistence exports.
The existing default-off `flora_shadow` key now compares native persisted records byte-for-byte and checks available public getters. An unavailable lot-tag getter skips only that getter comparison.
Tests cover independent ownership, byte narrowing, bit-exact positions, truncated records, trailing bytes and unavailable lot tags; bounded native record samples support diagnosis.
Final gate: `build-runs/final-flora-gate.log`; native verdict `build-runs/build-results/qbuild-20261025-065015-build-ctest.verdict`.
`floraocc-r4-shadow-2032 72/72` flora-record-shadow, mismatches=0; normal speed, clean to 2032-01-01, ref_warnings=0.
Same run: `flora_shadow 72/72`, `flora_shadow:flora-occupant 72/72`, `flora_shadow:flora-static 1/1`, all mismatches=0.
`floraocc-r4-replace-2032`: inherited 28-key full replacement profile, turbo, clean to 2032-01-01, ref_warnings=0.
Run evidence: `build-runs/game-results/<label>.verdict` and `<label>.log`; both runs used DLL SHA256 prefix `b9953dce`.
Missing: runtime flora occupants remain native; no flora class-registration changes or runtime ownership proof. Static-data runtime ownership replacement is also unfinished. Flora section 13.2 still prescribes native occupant creation; the complete derived runtime contract remains open.
Questions filed: none. C ABI exports changed; no ABI version bump. No coverage-map or regression-profile edits.
