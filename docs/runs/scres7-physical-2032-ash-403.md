tag: scres7-physical-2032-ash-403
run: 403
date: 2026-10-24
lane: glue-scres-replay7
dll: ff83377550ffc6df885501a03527188a59b16236a61ceb269842d4f62e880291
pacing: turbo
verdict: clean
switches: scres_shadow=1, scres_constructed=1, scres_file_shadow=1, turbo=1, end_date=2032-01-01; Konradshohe
result: scres 76/149; clean to 2032-01-04, ref_warnings=0, all problem counts zero, real_seconds=31.250. Existing natural command-token and constructed-adapter lines remain 76/76 exact. New file-parser replay has 73 dated lines, each mismatches=72, resyncs=0, refused=0. Physical reads=1107 (122 empty), false EOF-or-failure=2, opens=3, failed opens=1, commands=1061, recursive includes=1, inclusive native totals=1530. Two root replays: Graphics Rules events=2174, own physical=684, inclusive return=1107, differences=72; optional Graphics Options open failure events=2, differences=0. Diagnostic follow-up observes physical inputs with no command/include boundary; no native routing rule guessed from this count.

Unique request/verdict: `scres7-physical-2032-ash`.
Evidence: `build-runs/game-results/scres7-physical-2032-ash.{log,verdict}`.
Full gate before this run: Rust 6904/0 failed, clippy 0 diagnostics,
native build 240/240, CTest 74/74, qbuild all PASS.
No game request remains outstanding.
