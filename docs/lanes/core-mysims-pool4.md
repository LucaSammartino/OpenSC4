# core-mysims-pool4 — MySimPool / MySimDispatch job search to mysims §4.7.1 / §4.12

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-mysims-pool4 (rework mysims dispatch job search §4.7.1 and pool §4.12 cores) | `core-mysims-pool4` / `.claude/worktrees/core-mysims-pool4` | ready (tip 593558b5; gate PASS) | wire dispatch/pool cores into capi/plugin; answers to MS11/MS12 |

## Report

What changed (engine/core and engine/conformance only; no plugin, CAPI, CMake or ABI change):
- 33d17cad engine/core/src/mysims/dispatch.rs: job search per §4.7.1 (resetPath, flags 0x11, filter 0, wealth
  destination flags OR-accumulated in the dispatch's long-lived pathfinder, density 1, start rect from the home lot
  or cell, travel 0x1CF/0x1CF, one FindPath, end-cell lot / neighbour-city resolution with clamp);
  `set_current_commute_destination` (valid → store; notify false → false; state 2 → store unvalidated).
  engine/core/src/mysims/pool.rs: §4.12 (typed PoolHost, MySims.dat parse/encode, Import, cache reset, portrait
  display path, gender text). Tests dispatch_tests t30-t39, pool_tests p1-p7.
- b85e03c5 MemoryPoolHost; 593558b5 clippy fix (while-let record loop).
- 9b03f8df + 6104367c engine/conformance/src/mysims_pool.rs ops `mysim_job_search`, `mysim_pool`,
  `mysim_portrait`, `mysim_gender`; vectors ms12_21..ms12_27 (§4.7.1 worked examples, §4.12). The earlier failure
  was the vectors giving partial objects to the exact comparator; every `expect` is now the complete result object
  (all keys, derived from the spec), the comparator (scalar.rs `matches`, non-partial) is unchanged. README: "MySim
  ops" job-search/pool table and "Coverage (MySims §12)" rows for ms12_21..27.

Measured: `qbuild all` PASS: cargo test --workspace 3960 passed / 0 failed (baseline 3944), clippy 0, CMake build
PASS, ctest 64/64. fmt: lane-owned .rs files clean (lib.rs/mysims.rs carry pre-existing rustfmt diffs, untouched).
Game: none (dispatch job search and pool cores are not in the plugin or the mysims_shadow; shadow unchanged 73/73).

Owner coverage: cSC4MySimDispatch: job search §4.7.1 implemented + conformance (CheckCommute unchanged, trip
automaton not done), core only. cSC4MySimPool: §4.12 implemented + conformance modulo MS12, core only.
cSC4MySimAgentSimulator, cSC4MySim, cSC4MyStreetSim, cSC4MySimAdvice, cSC4MySimBalloonAdvice: unchanged.

Questions: MS11 (job-search location), MS12 (pool details) in docs/specs/questions/mysims.md, open; QUESTIONS.md
index not regenerated (coordinator runs the index tool).

Next: wire the dispatch job search and pool into capi/plugin (shadow keys), then a game run; trip automaton and
CheckCommute per §4.6/§7.2; revisit once MS11/MS12 are answered.
