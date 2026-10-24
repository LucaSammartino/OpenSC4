tag: mysimdispatch-shadow9-20261024-ctor02-495
run: 495
date: 2026-10-24
lane: glue-mysimdispatch-shadow9
dll: FreeCity4-glue-mysimdispatch-shadow9 (source 9e3c6df24, sha256 prefix de5c5034)
pacing: turbo
verdict: clean
switches: train187 house shadows from main build-runs/regress-train187-shadows.ini plus mysimdispatch_shadow=1; Konradshohe; end_date=2032-01-01; seconds=1500; timeout=1800
result: Reached 2032-01-01; clean shutdown, foreign_exceptions=0, ref_warnings=0, RTC=0, stalls=0, overruns=0, Rust panics=0, other WARNING lines=0; 820.547 real seconds, 839 s wall, slot 3. Exact line: mysimdispatch 73/73. First sample 2026-01-27, last 2032-01-01; all constructor=1, live_headers=0, fields=7, errors=0, mismatches=0, total 511 fields. Scope is the native constructor fixture and writer class/version only: no live dispatch record was discoverable and no live queue/trip/transport method was compared. MS20 blocks typed native observation/input replay; MS18 remains partial. This result must not credit cSC4MySimDispatch* whole. Corrected-source run is distinct from incomplete/audit-warning run 469. Final qbuild all: cargo 6986 passed, clippy clean, native build 207/207 edges and CTest 79/79.
