| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-camera-contracts4: CA3–CA6 native contracts | Codex / `re-camera-contracts4` (RE spec lane, detached worktree) | ready docs; CA4–CA6 answered DIFFERS; CA3 perspective-entry residual; question/index and whitespace checks PASS; ABI unchanged; game/shadow n/a | 1. Reconcile implementation with native differences. 2. Resolve CA3 perspective entry before any setter fidelity claim. 3. Require native shadow/replacement evidence before promotion. |

## Report

Completed the single bounded RE round. Changes are spec/report text only; engine, tools, ABI tables,
HANDOFF and README were neither edited nor used as implementation evidence. No subagents or REA calls.
Primary evidence is the Windows export instruction bytes and native PE words, with bounded decomp reads
as navigation aids. No core implementation or test expected values were read.

### Native contracts published

- **CA3, resolved portions:** camera constructor box is left/bottom −1 and right/top +1, near/far 1/2;
  remembered target and up both zero. Transform counters are bytes, projection counters u32, initially
  projection/cache 1/0. Look-at normalization has no degenerate fallback. SetOrtho increments even for an
  identical box. The perspective builder branch proves state evaluation, not a setter contract.
- **CA4, DIFFERS:** pre-init right and direction zero, up Y, clip distances 0/0. Detach retains initialised;
  external shutdown detaches/releases, and the destructor itself does not detach. Init remains idempotent
  after detach or a camera-less initial Init. Read sets loaded before checking version, writes directly
  through stream destinations, preserves partial mutations and raw malformed/NaN fields, and does no
  camera/city work. Invalid corner writes only y=0; invalid zoom accesses tables unchecked. SetZoom rejects
  NaN, but SetCustomMagnification can accept NaN from a valid old value and then cannot repair it.
- **CA5, DIFFERS:** integer-pair setter resets magnification before rejecting an unchanged pair; rotation
  alone preserves it. Tweak replaces the stored pair; its bounds update is conditional on emptiness.
  Viewport resize does not recompute control projection/eye/bounds. Messages synchronously call server-2
  slot 3, passing live field pointers while reusing the message object. Deferred copied events do not
  establish native timing. Invalid rotation commands return true without work. Float zoom equality is
  checked before truncation, so repeated 0.5 requests can both take the change path.
- **CA6, DIFFERS:** fixed directed half-planes, strict crossing/fraction selection, first edge on ties,
  length versus double 1e−8, float fraction thresholds, low-fraction zeroing, and opposite original-direction
  history stop. Zero-length outside moves stay outside; no outside-start edge projection or winding repair.
  Exhaustion returns original from. Target clamp starts at the rotation corner. Ground failure still sends
  updates: angle scroll retains its already-stored plane point, whereas target clamp retains old target
  and velocity. The old 0.48/lift formula and CA1 interpretation question are untouched.
- Corrected an additional constructor/projection discrepancy directly from bytes: yaw base is
  `0xBEC90FDB` (−π/8), not −π/4; default rotation-0 yaw is 3π/8. Updated dependent prose examples.

### Inventory and independent vectors

camera.md §7 explicitly accounts for native owner entries, all four camera and seven control vtable slots,
shared refcount bodies, misleading folded export aliases, out-of-region magnification getter and the
previously omitted generic update body at `0x007CCF80`. Associated ground/copy helpers are separated from
owner methods. No function-count-only completeness claim is made.

Added seven numbered discriminator families (10–16), a 12-row scroll table, and separate ground-failure
observations. These are analytic instruction-derived vectors, **not native captures**. Tiny-edge inputs
`0x322BCC77` and `0x322BCC78` straddle the double 1e−8 cutoff; examples distinguish fraction versus distance
insets and thresholds. The symmetric tie example explicitly cannot identify the first edge from final
coordinates alone. Exhaustion is specified as a conditional oracle, not an invented measured fixture.

### Validation and questions

- `python -X utf8 tools/questions.py index`: regenerated the question index.
- `python -X utf8 tools/questions.py check`: **PASS**; `git diff --check`: **PASS**.
- Native PE constants/vtables and cited x87 compare/store widths were inspected independently. No cargo,
  clippy or ctest run: docs-only lane, with engine reads prohibited. No game request or runtime capture;
  shadow exactness **n/a**. No ABI exports changed and no ABI-row proposals were needed.
- CA4–CA6 moved to `docs/specs/questions/answered/camera.md`, marked
  `ANSWERED 2026-10-22 (re-camera-contracts4): DIFFERS`. CA3 remains in the open file with one precise
  residual: identify a native perspective setter/call site and prove argument order/counter effects.
  The unproven pure-core setter remains an extension. CA1 remains open and unchanged.
- Updated only the coordinator-qualification paragraph of core-camera-control4's report: the earlier
  39 tests and compatibility smoke are not independent native camera evidence. Complete-owner and native
  replacement credit remain withheld pending implementation reconciliation and real native validation.

### Commits and next steps

`8cc181db`: first substantial defaults/read proof. `26e3247c`: lifecycle, command and scroll contracts.
`f17824e5`: native entry inventory, question reconciliation and coordinator qualification.
This final report is a separate documentation commit. The reserved worktree remains detached for the
supervisor to publish; no branch/ref updates, merge, push or self-restart were performed.

Implementers should reconcile the listed differences against the public spec, then pursue CA3's remaining
native entry evidence only if the perspective setter is needed. Do not turn safety containment of invalid
records, adapter shutdown/reset policy or deferred event capture into native fidelity claims.
