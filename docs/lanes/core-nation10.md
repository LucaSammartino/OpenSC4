| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-nation10: nation owner contracts round 2 | Codex / core-nation10 (detached HEAD) | ready: NQ1–NQ3 implemented; 7070 Rust tests, clippy, native and CTest 80/80 pass | Coordinator regenerates question index; resolve NQ4 non-ASCII case mapping; game adapter remains future integration. |

## Implemented

Module: `engine/core/src/nation`, owner table: its `README.md`.
Answer branch `re-nation-nq3` merged in `ffec414f8`; implementation `071d906c0`;
final engine source `4079ac421`. NQ1–NQ3 open entries were removed; their answered
copies remain intact. The additive core module conflict retained all modules.

- Process directory StaticInit guard, including no-app consumption and ignored
  directory creation failures; tuning reread on each new Init, repeated Init
  no-op, ordered subscriptions and Shutdown catalog/curve destruction.
- Immediate-root discovery in provider order, Downloads exclusion, directory and
  display-name setters before INI read, recursive overwrite/writable copies,
  localized installation templates and starter-region fallback.
- NUL-truncated 64-byte preference matching with case/length sensitivity, tutorial
  probes, initial index 0, same-index no-op, ordered selection and ignored callback
  failures. Borrowed getters take no COM references.
- Region-interface factory/add/delete operations, neighbor selection before
  catalog compaction, ignored filesystem failures, and explicit invalid-index,
  null-factory and LastRegion safety errors without rollback.
- Tutorial reset app/city guards, registry ordering, screen lifecycle, retention
  and recreation, resource exports, city read/move/shutdown/drop and active flags.
  Region::City supplies the concrete adapter type; signed positions preserve
  negative region cells, as foundations §11.2 requires.
- NQ3 missing-curve/difficulty outcomes, defaults, unsigned date evaluation and
  difficulty retention when reinitialization skips the empty demand-ID loop.

No nation C ABI/plugin adapter or ABI version change was introduced. No nation
serialization record exists in the spec, so no codec was invented. Owned handles
release on Drop; borrowed handles are never released by the core. Getters use
immutable access without RefCell.

## Final verification

`qbuild.py --wt <WT> all` on final source was collected to terminal exit 0,
`qbuild: PASS` (session 66935, native job
`qbuild-20261024-114016-build-ctest`). Engine contents stayed unchanged throughout;
only handoff documentation was committed while the native job waited.

- Cargo: **7070 passed, 0 failed, 0 ignored**, summed over all **46** test-result
  blocks in `build-runs/qbuild-test.log`.
- Clippy with warnings denied: **PASS**, exit 0, zero diagnostics;
  `build-runs/qbuild-clippy.log`.
- Native CMake build: **210/210 edges PASS**, 382.3 seconds;
  `build-runs/fc4verify-build.log`.
- Full CTest: **80/80 PASS**, zero failures, 412.2 seconds;
  `build-runs/fc4verify-ctest.log`. Native verification total: 799.8 seconds.
- Focused nation coverage: 3 core tests, 2 existing §5 vectors and 11 new owner
  vectors; final owner rerun **11/11 PASS**, including negative position transfer.
- Explicit-file formatting: **4/4 clean**; formatting fixes used apply_patch.
  Workspace declares Rust 1.75. Diff whitespace check: PASS.

Earlier evidence remains separate, not overwritten by the final success:

| Attempt / source | Result | Preserved evidence |
|---|---|---|
| 1 / 071d906c0 | Rust 7070 pass; clippy op_ref failed; native 212/212 and CTest 80/80 pass | `build-runs/r2-attempt1-*` |
| 2 / 1e450c22b | E0308 host/region city-type mismatch; test/clippy/native compile failed; CTest skipped | `build-runs/r2-attempt2-*` |
| 3 / ba5abdafb | Rust 7070, clippy, native 47/47, CTest 80/80 pass; later signed-coordinate review required a source fix | `build-runs/r2-attempt3-*` |

Attempt 3's tree-change note was documentation only. A standalone type diagnostic
is retained in `build-runs/nation-city-type-check.rs`. The initial q.py Bash
wrapper failed before Cargo (Win32 error 5); its log is
`build-runs/q/20261024-105358-34752.log`; subsequent focused Cargo ran directly.
There is no outstanding build operation or game request.

## Questions and integration boundary

NQ1–NQ3 are implemented. **NQ4 remains open** in
`docs/specs/questions/nation.md`: the encoding/case map for non-ASCII tutorial
directory names is unspecified. `lower_directory` comments the current choice:
fold ASCII A–Z and preserve other bytes. Preferences remain case sensitive.

The task requests `python tools/questions.py index`, but the lane's non-bending
clean-room rule prohibits running scripts under `tools/` and permits editing
only the question file under docs/specs. It was not run or replaced by a manual
index edit. **Coordinator must regenerate the index**; expected nation row:
**1 open, 3 answered, next NQ5**.

No game run was requested: this portable core is not wired to a game replacement
or shadow. Game verdict, shadow equality and ref-warning counts are unmeasured,
not claimed. A future C ABI/GZCOM adapter must implement the platform interfaces
using the game's services and perform in-game comparisons. The READY queue entry
names `engine/core/src/nation` and explicitly lists NQ4.
