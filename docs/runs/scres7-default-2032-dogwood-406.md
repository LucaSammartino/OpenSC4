tag: scres7-default-2032-dogwood-406
run: 406
date: 2026-10-24
lane: glue-scres-replay7
dll: 7e1bdf3e0d8436ad9374d3f6475f8d41d5ee24285c91cb4070e16aa76380be43
pacing: turbo
verdict: clean
switches: scres_shadow=1, scres_constructed=1, scres_file_shadow omitted/default-off, turbo=1, end_date=2032-01-01; Konradshohe
result: scres 76/76; clean to 2032-01-17, ref_warnings=0, all problem counts zero, real_seconds=29.656. Every original scres dated line exact, zero resynced lines. Natural command tokens 842/842 exact, file_calls=842, other_calls=0, lines=1061, expanded=0, nondispatch=219, paths=2, files=3, inclusive native file totals=1530, native exception boundaries=1, refused=0, mismatches=0. Constructed binary/text/resource-prefix groups remain exact 24/24, 22/22, 10/10. Same final DLL as combined run405. New physical-file path stays off by default, and no new file-control/raw replay lines appear. This verifies the existing scres profile, not the full house profile.

Unique request/verdict: `scres7-default-2032-dogwood`.
Evidence: `build-runs/game-results/scres7-default-2032-dogwood.{log,verdict}`.
Final full gate: Rust 6905/0 failed/0 ignored, clippy 0 diagnostics,
native build 194/194, CTest 74/74, qbuild all PASS.
No game or native request remains outstanding.
