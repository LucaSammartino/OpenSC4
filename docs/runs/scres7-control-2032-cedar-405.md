tag: scres7-control-2032-cedar-405
run: 405
date: 2026-10-24
lane: glue-scres-replay7
dll: 7e1bdf3e0d8436ad9374d3f6475f8d41d5ee24285c91cb4070e16aa76380be43
pacing: turbo
verdict: clean
switches: scres_shadow=1, scres_constructed=1, scres_file_shadow=1, turbo=1, end_date=2032-01-01; Konradshohe
result: scres 149/222; clean to 2032-01-04, ref_warnings=0, all problem counts zero, real_seconds=33.235. Partial file_control_replay 73/73 exact (mismatches=0), with foreign_preprocessing=45 explicitly supplied as an observed input; no block-comment grammar parity claim. Existing native command-token and constructed-adapter groups remain exact 76/76. Strict raw_file_parser_replay 0/73 exact, each mismatches=72, question=SCR11; no differences hidden or resynchronized. All replay groups resyncs=0, refused=0. Reads=1107, blank=122, false EOF-or-failure=2, opens=3, failed opens=1, commands=1061, includes=1, physical=1107, inclusive native totals=1530. Two roots replayed: Graphics Rules own physical=684, inclusive return=1107, raw differences=72, control differences=0; optional Graphics Options open failure raw/control differences=0. Natural command tokens 842/842 exact; constructed adapters 24/24 binary, 22/22 text, 10/10 shipped resource prefix.

Unique request/verdict: `scres7-control-2032-cedar`.
Evidence: `build-runs/game-results/scres7-control-2032-cedar.{log,verdict}`.

Representative counted lines (date 2026-01-27, load):

```
scres shadow 2026-01-27 load: partial=file_control_replay foreign_preprocessing=45 reads=1107 false_eof_or_failure=2 includes=1 inclusive_totals=1530 resyncs=0; mismatches=0
scres shadow 2026-01-27 load: partial=raw_file_parser_replay question=SCR11 roots=2 replays=2 reads=1107 blank=122 false_eof_or_failure=2 opens=3 open_failed=1 commands=1061 includes=1 physical=1107 inclusive_totals=1530 undispatched=45 refused=0 resyncs=0; mismatches=72
```

Final gate on the same engine source: Rust 6905/0 failed/0 ignored,
clippy 0 diagnostics, native build 194/194, CTest 74/74; qbuild all PASS,
queue `qbuild-20261024-043532-build-ctest` collected to terminal.
Whole cFileParser verification remains blocked on SCR11. Keep the new key
default-off and outside the house profile. No game request remains outstanding.
