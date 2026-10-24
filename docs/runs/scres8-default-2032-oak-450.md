tag: scres8-default-2032-oak-450
run: 450
date: 2026-10-24
lane: glue-scres-preproc8
dll: 0f40f2a589fad1fe2ca25f3389b52974f723ad88a593044c957c4472a3f45a94
pacing: turbo
verdict: clean
switches: scres_shadow=1, scres_constructed=1, scres_file_shadow omitted/default-off, turbo=1, end_date=2032-01-01; Konradshohe
result: scres 76/76; clean to 2032-01-16, ref_warnings=0, all harness problem counts zero, real_seconds=32.844. Existing native_command_tokens 73/73 dated lines and adapter_reads 3/3 exact; no raw_file_parser_replay lines emitted with the new key omitted. Every counted mismatch zero, zero resynced lines. Same DLL and unchanged engine source as raw run449. Full stable-source gate: Rust6936/0 failed/0 ignored, clippy0 diagnostics, native240/240, CTest74/74.

Unique request/verdict: `scres8-default-2032-oak-450`.
Evidence: `build-runs/game-results/scres8-default-2032-oak-450.{log,verdict}`.
The single event-driven wait completed normally; no qwait or replacement request.
Independent dated-line counting: **scres 76/76**. This checks the existing scres
profile and the new key default, rather than the complete house profile.
