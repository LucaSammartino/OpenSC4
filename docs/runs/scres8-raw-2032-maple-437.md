tag: scres8-raw-2032-maple-437
run: 437
date: 2026-10-24
lane: glue-scres-preproc8
dll: 0f40f2a589fad1fe2ca25f3389b52974f723ad88a593044c957c4472a3f45a94
pacing: turbo
verdict: clean
switches: scres_shadow=1, scres_constructed=1, scres_file_shadow=1, turbo=1, end_date=2032-01-01; Konradshohe
result: scres 149/149; clean to 2032-01-05, ref_warnings=0, all harness problem counts zero, real_seconds=34.015. Raw file-parser replay 73/73 dated lines exact, every mismatches=0 and resyncs=0; existing scres groups kept 76/76 (native_command_tokens 73/73 plus adapter_reads 3/3). Reads=1107, blank=122, false EOF-or-failure=2, opens=3, failed opens=1, commands=1061, includes=1, physical=1107, inclusive totals=1530, undispatched=45, refused=0. No observed suppression input, seeking or resynchronization. Native command tokens 842/842 exact; constructed binary24/24, text22/22, resource-prefix10/10 exact. Full stable-source gate: Rust6936/0 failed/0 ignored, clippy0 diagnostics, native240/240, CTest74/74.

Unique request/verdict: `scres8-raw-2032-maple-437`.
Evidence: `build-runs/game-results/scres8-raw-2032-maple-437.{log,verdict}`.
The single event-driven wait completed normally; no qwait or replacement request.
Independent dated-line counting: **scres 149/149**, raw73/73, kept76/76.
First raw sample is 2026-01-27 load; final raw sample is 2032-01-01 month.
SCR11 is implemented; there are no remaining numbered differences in this run.
The byte decoder, command execution/state, FileParser lifecycle/watch callbacks
and unrelated scres owners are not implied by raw file-loop equality.
