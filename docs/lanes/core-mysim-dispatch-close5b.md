| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-mysim-dispatch-close5b (dispatch settled MS18/MS19) | Codex / `core-mysim-dispatch-close5b` (engine lane, detached HEAD) | WIP: implementation commits fe37fbca and 784a20b7; core dispatch 27/27 and dispatch conformance 22/22 PASS; workspace 6093/6093 and clippy 0 diagnostics PASS twice; native configure TIMEOUT twice (900s each), CTest skipped; no game run; ABI unchanged; no new questions | (1) Resolve native compiler-probe stall and rerun full gate; (2) build DLL and run unique MySims shadow regression; (3) review/land settled contracts; residual MS18 remains open |

## Report

All contracts newly settled by MS19 and the published MS18 subset have code and
executable conformance coverage. This contribution is **WIP for validation**:
the required native gate cannot configure, so no game DLL or game verdict exists.
The complete dispatch owner still has the explicitly unsettled MS18 scope below.

### Implementation

- `fe37fbca`: exact location equality across both flags, occupant identity,
  cells and regional coordinates; world/timing ignored. Unconditional commute
  teleport copies invalid locations, uses wrapping timing/counters, writes all
  state before callbacks and increments counters after notification attempts.
  State-1 DispatchCommute teleports then replaces the queue with home.
- `784a20b7`: float32 asymmetric corner-to-rectangle geometry, signed linear
  distance versus wrapping squared tuning, strict/inclusive comparisons;
  ordinary mode-switch waits; first-purpose stop selection and all five mode /
  purpose / group mappings; interface-filtered boarding order, strict timeout,
  real foreign next-leg dispatch, stuck and separately notified success masks;
  final-marker observations and the separately selected common notification tail.
- The verified transport operations live in `dispatch/transport.rs` and extend
  `TripServices`. Remaining route-index and branch/mask selections are not guessed
  or delegated as policy to a foreign service. `animation_message` continues to
  report Unsupported for those unsettled selections. Accepted helper operations
  are not promises of trip completion or synthetic arrival.
- Updated both dispatch and cLocation owner-closure tables, including sections
  12.27 and 12.28. No plugin, C API, ABI, coverage-map or generated ABI-entry edits.

### Measured verification

- Five MS19 fixtures and five MS18 fixtures are executable in
  `engine/conformance/tests/mysim_dispatch.rs`. Three additional tests cover all
  81 ordinary/sentinel pairs, all stop/group mappings and boarding order, and
  final-marker/common-tail callback order. Total dispatch conformance: **22/22**.
- Focused core dispatch suite: **27/27**. Explicit-file formatting check:
  **PASS, 5 files, diff_lines=0**. `git diff --check` passed.
- Required `qbuild all`, R1: **FAIL** native configure after 900s at
  `Detecting C compiler ABI info`; workspace **6093/6093 PASS**, clippy
  **0 diagnostics PASS**, CTest skipped. R2 changed build input `_CL_=/Z7`:
  the same terminal native timeout; the same Rust/clippy pass counts.
- Failed configure evidence is preserved separately in
  `build-runs/failed-configure-r1.log` and `failed-configure-r2.log`;
  `gate-r1.txt` records the original result. Current Rust logs are
  `qbuild-test.log` and `qbuild-clippy.log` in that directory.
- A bounded configure diagnostic seeded generated `engine/build-x86/CMakeCache.txt`
  with explicit `/Z7` debug compile flags and `/DEBUG:NONE /INCREMENTAL:NO`
  executable debug link flags. Its first command-string invocation failed with
  Access denied (`native-probe-r3.log`); a batch invocation reached the same ABI
  probe and timed out (`native-probe-r4.log`). Its cleanup wait also timed out;
  the associated cmd/CMake/Ninja processes 30212/24304/14352 were subsequently
  stopped and confirmed absent. No successful native result is claimed.
- No redundant full check was started after that diagnostic failed. No game
  request was submitted, no game verdict or shadow count is claimed, and there
  is no outstanding owned build or game request. The existing MySims shadow
  excludes dispatch trip state; a future run is regression evidence only.
- Earlier corrected failures: q.py could not start Git Bash (CreateFileMapping
  error 5; `build-runs/q/20261023-174301-17120.log`), so focused Cargo used direct
  invocation. The initial MS18 geometry fixture unintentionally resolved an
  overlapping lot; the corrected point fixture and all 22 tests pass. Initial
  relative fmt paths failed; the absolute owned-file invocation passed.

### Questions and next steps

No new question was needed. **MS19 has no remaining Unsupported path.** Existing
MS18 in `docs/specs/questions/mysims.md` remains the only specification blocker:
sections **4.6.1/4.6.2** complete route branch/mask matrix, route-index selection,
idle/FE/FF/cancellation, neighbour retry enumeration/index and complete
completion/recovery ordering (including EBA06BD0); **7.2.1** dispatch-tool/pylon
input lifecycle and mission callbacks; **8** delegated automata/model RNG.

Resolve the native toolchain stall, review/reset the diagnostic generated cache
as appropriate, and run `python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py
--wt C:/Users/tlori/FreeCity4/.claude/worktrees/core-mysim-dispatch-close5b all`.
Then request a unique MySims shadow regression with the resulting DLL, collect
the actual verdict without qwait, require clean/ref_warnings=0 and exact shadow
(or numbered differences), and promote this lane from WIP only when validated.

Worktree remains detached by supervisor instruction. No refs were changed.
Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-mysim-dispatch-close5b-r1-manifest.json`.
