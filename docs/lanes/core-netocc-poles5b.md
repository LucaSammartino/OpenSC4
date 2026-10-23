| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-netocc-poles5b: utility owner follow-up | Codex / detached `core-netocc-poles5b` | wip: implementation committed; focused 69/69, workspace 6087/0, clippy clean, fmt clean; full gate incomplete at stalled CMake compiler ABI probe | Collect a complete CMake/CTest gate in a working build environment; retain specification-open owner rows. No ABI changes or game runs. |

## Report

Implementation milestone: `d2fd5b25`. All newly settled behavior is implemented
and tested. Complete native owner closure remains limited by the questions
listed below; this delivery makes no claim to implement those open algorithms.

### Changes

- NO44 / network §6.1: `Pylon::fresh` initializes UI=1 and dispatch state=0,
  preserving an explicit incoming dispatch-type word without uninitialized Rust
  memory. Existing `new(states)` is documented as a supplied live-state seed.
- Network §6.2.2: cached local/world/occupant bounds, texture-ID sorting,
  transform copy/counter/mask semantics, UI-gated ray-box picking and strict
  distance replacement. Getters never rebuild or clear dirty state.
- Network §6.2.2: both ordinary texture tables, release/null of an old binding
  versus release of a raw ID, binding acquisition and stale-ID preservation on
  replacement failure. MySim composition remains explicitly unsupported (NO36);
  out-of-table states remain unsupported exactly as the spec says.
- Network §3.6.2 / underground §4: standalone connection floor/wall generation,
  attached early exits, clearing versus appending, ordered endpoints, retained
  alpha, white RGB and the shared endpoint-A drop for both bottom wall UVs.
  Builders consume the published model-state latch; bounds/notification beyond
  that boundary still report the existing UG5/NO40 unsupported result.
- Rechecked the other owner rows. Added the previously missing specified shared
  texture reload path and creation's fresh-versus-loaded reference cleanup,
  stored class ID and static descriptor identity. Added focused tests for these.
- Updated the module README with separate closure sections for power pole,
  `::tConnection`, pylon and underground connection. Existing settled pole
  behavior was rechecked through the specification and existing focused tests.

Seven new executable tests cover conformance cases 45–50 plus connection shared
texture reload; the creation test now covers both nonnull preexisting refs and
idempotent creation. No plugin/capi/ABI or coverage-map edits were made.

### Validation and evidence

- Focused final-source tests: **69 passed, 0 failed**.
  `build-runs/network_occupants--tests_.log`.
- Required `qbuild.py --wt <WT> all` invoked once on final Rust source:
  workspace **6087 passed, 0 failed** across 30 suite results;
  clippy **clean** with the required warnings gate. Evidence:
  `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`.
- **Full gate incomplete:** CMake identified MSVC C/C++ 19.37.32825, then stalled
  at `Detecting C compiler ABI info`. Configure log last advanced at 17:44:23;
  it was still unchanged at 17:52:24. After further waiting, interrupted the
  same operation and collected its terminal result (exit 1, session 37472).
  No duplicate gate was launched. CTest was not reached.
  Evidence: `build-runs/fc4verify-configure.log` and generated
  `engine/build-x86/CMakeFiles/CMakeConfigureLog.yaml`.
- Owned-file formatting: **4 files, exit 0, diff_lines=0** using absolute paths.
  `git diff --check HEAD^ HEAD` passed for the implementation commit.
- Initial q.py failed at Bash startup (Win32 error 5); its failure is retained in
  `build-runs/q/20261023-173649-15120.log`. Direct Cargo execution succeeded.
  Initial fmt used relative paths, which qbuild could not resolve; corrected to
  absolute paths. Neither attempt is represented as a passing build check.
- Game/shadow: **n/a**, core-only delivery with no game wiring or shadow key.
  No game request is outstanding. No C ABI export changed.

### Remaining specification limits and next steps

No new questions were needed or filed. Existing questions remain in
`docs/specs/questions/network-occupants.md` and
`docs/specs/questions/underground-occupants.md`:

- **NO33/NO43:** network §§4.1–4.3 native pool/full initialization and model
  lifecycle; §4.6 full wire construction/cell walk; §4.7 full position/model/
  zoom/bounds/transform/Pick/Draw; §4.7.1 diagonal floor numeric operation order.
  Unrotated floor/walls and the previously settled pole contracts remain tested.
- **NO36:** network §6.2 full RebuildModel/billboard construction, MySim and
  call-label composition, Draw and DoMessage. Constructor, cached accessors,
  ordinary textures and Pick are no longer open. The existing §1.2 pylon QI
  discrepancy remains **NO10**; no identity behavior was invented here.
- **NO40/UG5:** network §§3.6/3.6.1 descriptor-dependent piece/geometry dispatch,
  ComputeBox and its renderer notification dependency, complete Draw, view/
  transform and message decisions; underground §4/§9 test 12 has the same
  remaining bounds/notification boundary. Standalone §3.6.2 builders are settled.
- **NO41/NO42:** unknown saved-word/flag interpretations remain opaque; their
  specified serialization and newly specified assignments are implemented.

Next: run a complete CMake/CTest validation in an environment where Ninja's
compiler ABI probe completes, then change the queue state to ready. Specification
work on the questions above is still needed for complete native owner closure.
