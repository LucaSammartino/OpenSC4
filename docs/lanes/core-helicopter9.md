| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-helicopter9: Rust cSC4Helicopter | Codex / core-helicopter9 | ready: settled contracts covered; gates pass; AF2–AF6/AF9 open | Review and land; claim owner module; resolve open flight contracts before movement shadow |

## Implementation

Implementation commit: e60c21f25. Owner module:
`engine/core/src/helicopter/mod.rs`; closure table:
`engine/core/src/helicopter/README.md`. The coordinator can use that module
path for the implemented column. `automata::Helicopter` re-exports the same
owner; the legacy duplicate class was removed.

Implemented aircraft §4.1's resolved thirteen-field named binary32 importer,
nearest whole-record selection, zero-preserving preset overlay, exact
conversion constants/order and non-finite/negative-value handling. Unknown
constructor preset fields remain explicit rather than guessed. Corrected the
numeric goal default to PopState, including CROP_DUST and every unlisted type.
Preserved known chase rejection/geometry, independent flight phases and the
6000-ms mission LOITER push. Added explicit error boundaries for the named
methods whose laws are open in AF2–AF6.

Reused the shared automaton/aircraft codecs, home-position capture/restore,
class identity, altitude and world-margin helpers. Tests include same-input
byte-identical record round trips, concrete/base version gates, every truncated
read boundary with atomic owner preservation, and the existing nested-write
failure check. No C ABI, plugin, ABI version or coverage-map changes.

## Validation

- Focused conformance: 14 new helicopter tests plus 16 existing aircraft tests,
  **30 passed, 0 failed**. Covers aircraft §8 vector 7 and the helicopter
  portions of automata §14 vectors 15/31.
- Explicit-file qbuild fmt: **PASS**, 5 Rust files, zero diff lines.
- Required qbuild all on final source: **PASS**, summed workspace results
  **6998 passed, 0 failed, 0 ignored**; clippy **PASS**, zero diagnostics.
  Rust logs: `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`.
- All's queued native stage `qbuild-20261024-084413-build-ctest`: **PASS**,
  build 253/253 edges (107.9 s), CTest **79/79** (241.1 s), total 353.5 s.
- Separately requested qbuild verify mode
  `qbuild-20261024-090152-build-ctest`: **PASS**, build 46/46 edges (2.0 s),
  CTest **79/79** (81.6 s), total 85.6 s. Latest full native logs:
  `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`.
- Both asynchronous queue operations were collected to terminal PASS; no
  outstanding request. No source changes followed the passing full gate.
- Initial focused compile diagnostics were corrected before the full gate;
  evidence is retained in `build-runs/focused-first-failure.txt`.
  The q.py launcher failed before cargo due to Git Bash mapping permissions;
  its log remains `build-runs/q/20261024-083505-26160.log`.

## Questions and exact open sections

New **AF9** is appended to `docs/specs/questions/aircraft.md`: exact initial
rotor preset bits and post-Read preset/phase reconstruction (AF4 follow-up).
The coordinator should regenerate the question index; this lane only appended
to the permitted question file.

All settled helicopter contracts are implemented. These owner sections remain
explicitly open, with no invented model or successful stub:

- **aircraft §4.1:** initial/post-Read preset values, AF9 / AF4. Unknown fields
  are represented as None; a supplied preset and every settled overlay rule work.
- **aircraft §4.2:** rotor force/integration/attitude/collision laws, AF2;
  complete AI controls and goal transitions/claim failures, AF3;
  native initialization/failure cleanup and sound mapping, AF4;
  player parameter selection, commands/weapons and spotlight cone updates,
  AF5; messages/dispatch transitions, AF6.
- **automata §6.6:** complete handled-goal bodies, chase distance/teleport/control
  gates, AF3, and phase-to-sound actions, AF4. Explicit rejection, lead/hover
  geometry, default pops, phase no-op and mission timeout facts are covered.

## Game and next steps

No in-game request was made: this is core-only and deliberately unwired.
A narrow read-only shadow is feasible next after an adapter is added:
compare the thirteen imported fields and pre/post preset bits; saved versions
and home vectors; phase no-ops; invalid-chase/default goal pops. Exact flight
position/velocity comparison requires AF2/AF3 answers first.

Coordinator: review/land, claim the owner module path above, and route AF9 with
the existing AF2–AF6 questions. No game request or native check remains pending.
