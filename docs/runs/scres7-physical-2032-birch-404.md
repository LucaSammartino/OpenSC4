tag: scres7-physical-2032-birch-404
run: 404
date: 2026-10-24
lane: glue-scres-replay7
dll: 3ce78a0818996dae5e5c81d466d0f80e545f324042f7781e52fa50d93ae6cfae
pacing: turbo
verdict: clean
switches: scres_shadow=1, scres_constructed=1, scres_file_shadow=1, turbo=1, end_date=2032-01-01; Konradshohe
result: scres 76/149; clean to 2032-01-15, ref_warnings=0, all problem counts zero, real_seconds=29.203. Existing scres comparisons exact 76/76; raw file replay 73 dated lines each mismatches=72, resyncs=0, refused=0. Reads=1107, empty=122, false EOF-or-failure=2, opens=3, failed opens=1, commands=1061, includes=1, native inclusive totals=1530, undispatched physical reads=45. Bounded diagnostics locate first suppressed physical input at Graphics Rules line 216, bytes #<; subsequent suppressed inputs include blank lines, explanatory text and assignments. SCR11 filed for missing block-comment preprocessing grammar; no grammar inferred. Graphics Rules own physical=684, child=423, root return=1107; optional Graphics Options failure exact (2 events).

Unique request/verdict: `scres7-physical-2032-birch`.
Evidence: `build-runs/game-results/scres7-physical-2032-birch.{log,verdict}`.
Diagnostic full gate: Rust 6904/0 failed, clippy 0 diagnostics,
native build 47/47, CTest 74/74, queue qbuild-20261024-042645-build-ctest PASS.
The differing inputs are captured during startup before turbo begins; this is
a missing grammar contract, not a difference observed only under turbo.
No game request remains outstanding.
