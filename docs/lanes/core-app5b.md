| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-app5b (application owner AP12–AP15) | Codex / core-app5b (detached HEAD) | wip: implementation complete; cargo 6102/6102 and clippy PASS; CMake blocked by Ninja subprocess launch hang | Restore build-launch environment, rerun full gate; resolve AP16 and regenerate question index |

## Report

Implemented the cSC4App owner contracts settled by AP12–AP15 in the existing
`engine/core/src/application` module. The module README maps every spec section
to code/tests or an explicit open question and maps §16 conformance vectors to
executable tests. This lane is WIP solely pending successful CMake/full CTest
verification; no complete native application boot is claimed.

### Changes

- `44993a41`: exact Admin Enabled comparison and plugin dispatch/preconditions;
  settled interface/path getters; ordered video mappings with byte/unknown-value
  preservation; preference pause/language/window/driver/speed/cheat policies.
- `8d5e3f11`: Resources.ini hexadecimal parsing, first-semicolon split, malformed
  entries and nested condition groups; all eight source passes, enumeration
  ordering/filters, failure cleanup and fifteen factory registrations; published
  font/default/style/control-colour/picture/cursor/caption tables and ordering.
- `0652732a`: save filename sanitation, encoding, cap, collisions and existing-city
  ID reuse; obliteration's destructive writer failure, ignored read-init failure
  and independent view restoration; terrain slot-29 path; resource/preference/INI
  integration into the owned runner; closure/conformance documentation.
- `f6e89878`: corrected two diagnosed Clippy warnings without behavior changes.
  `33c3b337` documents the corrected Rust gate milestone and AP16 table entries.

Foreign framework/window/renderer/simulator/filesystem primitives remain traits.
No plugin or capi wiring, C ABI exports, ABI version or generated ABI entries
changed. Saved preference bytes outside modeled fields remain preserved.
`CheatRequests::step` exposes the specified uppercase-X nonprogress defect without
an infinite blocking core call. Native-invalid cases return typed results.

### Measured verification

- Focused milestones: 54, 57, 64 and 68 application tests passed in the retained
  `build-runs/focused-{second,resources,ui,integration}.log` files. Earlier
  compile/fixture failures remain in their distinct original evidence files.
- Explicit owned-file formatting checks passed; the two Clippy-fix files also
  passed `qbuild fmt` with zero diff lines.
- First full-gate evidence is preserved in `build-runs/gate-r1-clippy-failed/`:
  6102 workspace tests passed; Clippy found collapsible_if/type_complexity.
  The inherited exec session 94508 was no longer registered on continuation.
- Corrected full `qbuild all`, session 32931, reached terminal **FAIL**:
  **6102 passed / 0 failed / 0 ignored**, summed across all 30 workspace test
  result lines; this includes **70 application tests**. Clippy: **0 diagnostics**.
- CMake configure stalled at `Detecting C compiler ABI info` for 842 seconds.
  Its Ninja compiler-probe child had no child compiler process. Ninja --version
  and a dry run of the exact target succeeded; a separate no-op Ninja target
  (`cmd.exe /C exit 0`) timed out after 15 seconds. Only this lane's stalled
  CMake process and its remaining Ninja child were stopped. The original gate
  then reported configure FAIL (841.5 seconds) and **CTest skipped**.
- Logs: `build-runs/qbuild-{test,clippy}.log`,
  `build-runs/fc4verify-configure.log`, and
  `build-runs/gate-r2-infrastructure-evidence.md`. Reproducer:
  `build-runs/probe-ninja.py` plus `build-runs/ninja-probe.ninja`.
- The verifier also noted tree changes during the run: these were documentation
  commits only. Rust source stayed at f6e89878 throughout. This is not a passing
  full gate, and no duplicate full check was launched while one was outstanding.

Game runs/shadow: n/a, no game requests or outstanding sessions. This change
only implements portable core policies and has no native integration to exercise.

### Questions and open sections

New **AP16**, appended to `docs/specs/questions/application.md`: §3.1 step 8
does not state whether Init consumes SetupResources' §14.2 count>=3 result.
The code comments and README record literal call-and-continue; the direct
resource API returns the specified boolean. The coordinator must regenerate the
question index; this engine lane cannot edit that generated spec file or run
the index generator under the clean-room rules.

Existing **AP3–AP11** remain open: §1/11.15 face identity (AP4); §2/9 aliases
(AP3), §9 append details (AP7); §3–5/12 startup/defaults/failure/destruction
details (AP6), §3.1/5.2 config/service meaning (AP5); §4.2 unnamed foreign
services (AP10); §6.1 GP/focus semantics (AP9); §8 parsing/acceptance edges
(AP8); §8/13.2 opaque preference meanings (AP11). Their already implemented
literal portions and remaining boundaries are listed individually in the module
README. No settled AP12–AP15 behavior remains merely Unsupported.

### Next steps

1. Restore Ninja subprocess launching in the build environment; the no-op
   reproducer isolates the obstruction from application source/compiler cache.
2. Run `python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py --wt C:/Users/tlori/FreeCity4/.claude/worktrees/core-app5b all`
   on this committed source, collect the same operation to terminal, and require
   CMake/full CTest PASS before marking ready. Keep documentation unchanged during
   the gate to avoid the verifier's tree-change marker.
3. Regenerate the questions index and resolve AP16; AP3–AP11 need specification
   work before a complete native-owner claim.

Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-app5b-r2-manifest.json`.
