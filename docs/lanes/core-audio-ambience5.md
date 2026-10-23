| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-audio-ambience5 (portable ambience manager) | Codex / `core-audio-ambience5` | ready; cargo 5864/0, clippy clean, CTest 68/68; 46 focused tests | Review/land; resolve AA10, AA12, AA13; regenerate question index |

## Implementation

Closed the portable `cSC4AudioAmbienceManager` owner in
`engine/core/src/audio_ambience`. The existing `audio::ambience` implementation
was migrated and retained as a compatibility re-export, avoiding two owners.
Implementation commits: `f306d5c6` and final audit corrections `220b9dfa`.

The module covers lifetime, occupants, conditions, automata, alarms, tuning,
attention rectangles, the prime-step block walk, events, focus and dispatch.
Layers remain a foreign `AmbienceLayer` trait. Host boundaries now explicitly
cover timer restart, filter creation and the process-wide startup rectangle flag.
The walk makes progress even when its budget is already exhausted. Null sound
occupants and null events are handled, preferences are read after each successful
layer init, and zooms outside 0–4 preserve the previous rectangle.

The [module README](../../engine/core/src/audio_ambience/README.md) contains the
Owner closure table and maps every manager conformance vector. §10 vectors 9–18
describe `cSC4AudioAmbienceLayer` internals (§§8.1–8.7), which are foreign and are
not claimed by this owner. The sole Unsupported manager case is **§3.6 alarm
creation without a clock (existing AA10)**. It produces an explicit
`Unsupported::AlarmCreationWithoutClock`, rather than manufacturing an alarm.
All remaining manager behaviour is implemented, including literal choices for
the clarifications AA12 and AA13 below.

## Validation

Final command: `python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py --wt C:/Users/tlori/FreeCity4/.claude/worktrees/core-audio-ambience5 all`.
It ran with a stable tracked tree at `94789513` (source `220b9dfa`) and was
collected to actual terminal **qbuild PASS / fc4verify PASS**, without a
tree-changed warning. Unified session 21991 is finished.

- Cargo: **5,864 passed, 0 failed, 0 ignored**, summed over all 27 `test result:`
  lines in `build-runs/qbuild-test.log`.
- Clippy: **0 diagnostics**, workspace/all targets with `-D warnings`,
  `build-runs/qbuild-clippy.log`.
- CMake: **PASS**, all 4 required build edges, 69.5 s,
  `build-runs/fc4verify-configure.log` and `fc4verify-build.log`.
- Full CTest: **68/68 passed**, 0 failed, 74.3 s,
  `build-runs/fc4verify-ctest.log`. Native verification total: 183.0 s.
- Focused owner tests: **46/46 passed**,
  `build-runs/q/20261023-121815-39304.log`.
- Owned-file fmt passed for `audio_ambience/mod.rs`, `audio_ambience/tests.rs`
  and the compatibility `audio/ambience.rs`; no shared-file reformat.

Earlier evidence is kept separate: the first focused run had 41 passes and one
old cell/world expectation failure (`build-runs/q/20261023-115552-41184.log`);
the corrected run passed 44/44, then final audit tests increased this to 46/46.
The initial relative-path fmt invocation failed to locate files; absolute paths
fixed that input (`build-runs/initial-fmt-failure.txt`). The first full check
passed 5862/0, clippy and CTest 68/68 but reported a changed tree after the audit
corrections; it is superseded by the final check. Its summary is preserved in
`build-runs/first-gate-record.txt`.

Both native checks stalled in their own `sccache --show-stats --stats-format json`
query (private cache port 4356). Only the verified statistics-query children
15728 and 40624 were terminated, exit 15; compilers, cache server and verification
operations were left running. Both original operations then reached terminal
PASS. The ignored helper `build-runs/own-gate-processes.py` restricts recovery to
this worktree's qbuild descendants and queries older than 30 seconds. WMI process
inspection was denied; existing psutil could inspect these own processes.

No game request was submitted: this portable module has no plugin/capi wiring.
Game verdict, ref-warning count and shadow comparison are therefore **n/a**.
No C ABI exports or ABI version changed.

## Questions and next steps

Appended to `docs/specs/questions/audio-ambience.md`:

- **AA12 (§§5, 8.4):** clarify the coordinate type of `BeginUpdate`. Implemented
  literal world-space attention rectangle, with a precise forwarding test.
- **AA13 (§3.3):** clarify alarm matching and duplicate removal. Implemented the
  first occupant/kind match in list order, with a regression test.

Existing **AA10 (§3.6)** remains open only for absent-clock creation policy.
The coordinator should review/land this owner, regenerate the question index
(the lane may only append its question file), claim coverage after review, and
route AA10/AA12/AA13 to the spec/product owners. Plugin integration and real-game
shadow work remain future work outside this lane's scope.
