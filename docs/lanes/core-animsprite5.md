| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-animsprite5: animated sprite owner | Codex / `core-animsprite5` | wip: owner spec gaps; CMake probe stalled | Resolve AS1/AS3/AS6/AS7/AS10/AS11 and build environment, then complete numeric conformance and full gate. |

## Report

The existing `engine/core/src/animated_sprite` module was extended in place.
Complete native owner closure is **not claimed**. Its README has one Owner closure
row per spec section, including all six §10 conformance items, with code/test
names and explicit question boundaries. No plugin/capi wiring, C ABI change,
coverage-map edit, original-owner callback, unsafe backend or game request.

Implemented state and transaction contracts include constructor defaults and
reference-count wrapping/destruction, transform copies/counters and placement
schedules, raw-byte playback and controller-snapshot stepping, repeated-load
prefix rollback, resource ownership, raw selection/packing, cached drawing and
stale picking. Rendering/resource/picking primitives remain foreign interfaces.
Round 2 fixes the shared 20-byte draw record: initialized shade/alpha words,
flags/highlight/fade aliases, opaque-byte preservation and arbitrary supplied
float-word preservation. Snapshot import documents which duplicated fields win.
LookAt rejects unsupported final cross products before committing state.

The safe dyadic numeric carrier accepts exact transients/stores; nonzero hardware
trig and inexact arithmetic return Unsupported. Signed-zero examples use
conditional nearest-even rules. This is not a qualified native numeric backend.
Legacy decoded-table/billboard helpers are explicitly separated in the README
from the raw owner path's contracts.

## Validation

- Focused tests: **44 passed, 0 failed**, `build-runs/focused-animsprite5-r2.log`.
- Explicit-file formatting: all 12 module .rs files passed; the later lifetime
  lint correction also passed its formatting check.
- Final `qbuild all`, stable source `c06897f7`, session 87902 collected to terminal:
  **cargo 5914 passed, 0 failed, 0 ignored** (sum of 27 result lines);
  **clippy -D warnings passed, 0 diagnostics**; **CMake configure failed,
  CTest skipped (0 executed)**. Overall **FAIL**.
- Final logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-configure.log`; terminal summary preserved in
  `build-runs/gate-animsprite5-r2-terminal.md`.
- CMake/Ninja repeatedly stalled in its compiler ABI probe before any build
  files were generated. No compiler child was running; the first Ninja consumed
  approximately 0.016 CPU seconds over eight minutes. Only this lane's stalled
  configure/probe processes were stopped. The final configure failure was
  collected after 70.9 seconds; it is not a passing build or CTest result.
- The first full gate had the same cargo total, a lifetime.rs clippy diagnostic
  corrected by `saturating_sub(1)`, and a stopped configure after 636 seconds.
  Its logs/configure YAML remain under `build-runs/gate-r2-first/`. A README edit
  during that first run also marked its tree changed; the final run had no edits.
- Earlier focused failures remain in `build-runs/focused-animsprite5-r2-failed.md`
  and round-1 `build-runs/q/` logs. No asynchronous operation or game request remains.

## Open scope and questions

Appended **AS10** (LookAt and Draw-angle primitive/spill boundaries) and **AS11**
(foreign factory IID/transfer) to `docs/specs/questions/animated-sprite.md` in
round 1. No additional round-2 question was necessary. The generated question
index is untouched under the lane's spec-edit/clean-room restriction.

The following sections remain qualified/open, not complete-owner credit:

- **§1.1, §3.2–3.3:** unpublished initial corner data and exact native
  allocator/thunk/function inventory reconciliation (AS3; residual AS9).
- **§2:** the non-sprite foreign factory acquisition contract (AS11).
- **§2.1, §4.1:** general numeric/malformed resource qualification
  (AS4/AS6/AS7); normal load transaction and raw packing rules are implemented.
- **§3.1–3.1.4, §4.1–4.2, §4.4, §5–5.1:** hardware trig, whole-call PC/RC,
  nonfinite/trap behavior and inexact arithmetic qualification (AS6/AS7).
  LookAt/Draw angle boundaries additionally require AS10.
- **§4.3:** native controller allocation/provenance (AS1); explicit snapshot
  state and stepping do not invent native creation or lifetime.
- **§10.2, §10.5–10.6:** general direction/orientation goldens, nonzero R2-R1/R2/R3,
  R3-C/L and full nonzero R3-M execution depend on the above numeric gaps.
  Q1–Q5, clock/loop, zero masks and conditional stage/zero examples have tests.
- **§7:** shipped usage attribution remains AS2, not executable owner behavior.

Next: repair the stalled compiler-probe environment; obtain the above spec
qualification; implement the newly settled numeric/controller/factory contracts;
run their independent conformance vectors and a passing full gate before promotion.
The merge-queue entry is WIP. Reserved branch `core-animsprite5` is published by
the supervisor; this round committed normally on detached HEAD.
