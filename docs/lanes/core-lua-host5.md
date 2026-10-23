| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lua-host5 (cGZLua5 owner closure) | Codex / core-lua-host5 (detached, supervisor publishes) | wip: 6094 tests pass, clippy clean; CMake configure timeout, CTest skipped; owner closure blocked by LH4/LH5 | Resolve LH4/LH5 and compiler-probe environment; complete lifecycle/commands before claiming owner coverage |

## Report

Extended the existing `engine/core/src/lua_script_server` module with `vm`, not a
second interpreter. `LuaApi` exposes typed Lua 5.0 C API, allocation, precision,
pointer-storage and logging primitives. Core owns base Open flag/order/default
reporter behavior; stack/global/registry wrappers; protected calls and traceback
insertion/removal; do-functions and alert/stderr routing; FTOL conversions; boxed
userdata pointers; tag/metatable callbacks and the process-wide tag counter;
reference and GC surfaces; and precision restoration (except deliberately
unguarded Unref). The cSCLua GetN convenience algorithm shares the stack session
but is explicitly not attributed to an extra Thread vtable slot.

`CommandGenerator` implements specified invalid-input checks, registration,
borrowed main-thread handling, Lua global validation/creation, endpoint retention
and Shutdown release order. It stops explicitly Unsupported at the unspecified
setup tail rather than claiming successful activation. No original host owner
operation was used as an interpreter primitive. No plugin/C ABI exports or
version changes; no coverage-map changes; no game requests or shadow claims.

The module README has a per-owner, per-major-spec-section matrix naming the
implementation/tests and all open boundaries. This is **not owner closure**:

- **LH4**, lua-host §§1.2/1.3, 3.1/3.2, 6.1/6.2: native constructor/refcount
  destruction, main and child IDs, registry/root lifetime, factory and transfer
  failures, GetThread ownership, resume/results and thread lifecycle/status/pause.
  Thread creation/destruction/lookup/resumption and unspecified lifecycle methods
  return Unsupported. Main-thread numeric ID is not invented. The literal base
  Open retains the state on failed registry allocation; this choice is questioned.
- **LH5**, lua-host §§1.2 and 6.3.2: command-generator constructor/destruction,
  generic/repeated Init, parameter creation, remaining setup and generated
  callbacks/command execution. The specified setup prefix is implemented, but
  activation and command generation stay Unsupported.
- Default reporter exact text remains the spec's existing §15 Q8; the chosen
  engine-log output labels an unavailable numeric ID LH4.

Both new questions are appended to `docs/specs/questions/lua-host.md`. The
coordinator must regenerate the question index; this engine lane did not run the
forbidden `tools/` scripts or edit the generated index.

## Validation and evidence

- Focused integration target `lua_vm`: **14 passed / 0 failed**. Tests execute
  the actual core tag callbacks and wrappers against a stack/table fixture.
  Compiled-chunk execution/arithmetic is a scripted foreign outcome, not a claim
  to implement or validate the Lua interpreter. T10/T11/T13/T18 are covered with
  that boundary; T17 base behavior is distinguished from cSCLua subscription.
- Explicit owned-file qbuild fmt: clean for all five new Rust files; the final
  clippy correction's test file was checked again and remained clean.
- First `qbuild all` on b415d6fc: **FAIL**, 6094 workspace tests passed / 0 failed
  across 31 result lines, one clippy fixture initialization diagnostic, CMake
  configure timed out after 900 seconds at `Detecting C compiler ABI info`;
  CTest skipped. Full first-run logs remain in `build-runs/gate-first/`.
- Commit 0e2d7d20 fixes that clippy diagnostic using struct initialization.
- Second `qbuild all` on the corrected stable Rust source: **FAIL**, **6094/0**,
  **clippy exit 0, diagnostics 0**; CMake again timed out at compiler ABI detection
  after **900 seconds**, CTest skipped. Full current logs:
  `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-configure.log`.
  The verifier also flags tree changes: the handoff HEAD and README attribution
  documentation changed during the wait; no Rust/C++ source changed. This run is
  not represented as a clean full gate.
- Configure diagnostic: the first generated compiler probe used `/Zi` and
  `/debug`. An ignored local `engine/build-x86/CMakeCache.txt` now seeds
  `CMAKE_TRY_COMPILE_CONFIGURATION=Release`; the second probe confirmed `/O2`
  and `/INCREMENTAL:NO`, without `/Zi` or `/debug`, but still timed out. This did
  not bypass compiler checks or alter committed CMake/plugin inputs.
- Both asynchronous full checks were collected to their actual terminal result;
  no request/check remains outstanding. No redundant third full run was started.
- The initial q.py invocation failed to start Git Bash (CreateFileMapping error
  5); direct PowerShell cargo worked. Its evidence is
  `build-runs/q/20261023-175822-21212.log`.

Implementation milestones: fe9f02e8 (owners), b415d6fc (conformance and matrix),
0e2d7d20 (clippy fix), d80dc2a6 (configure diagnostic/handoff).

## Next steps

Resolve LH4/LH5 before completing these owners or claiming their coverage. Run
the full gate in a working compiler-probe environment; there is no CMake/CTest or
game verification for this lane. The reserved branch is `core-lua-host5`; the
supervisor publishes the detached commits. Manifest:
`C:/Users/tlori/FreeCity4/build-runs/run4-core-lua-host5-r1-manifest.json`.
