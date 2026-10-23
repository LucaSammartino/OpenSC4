| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-animation-tick-manager4: portable tick manager, first substantive implementation | Codex; core-animation-tick-manager4 | wip: focused Rust passes; checker terminal pending | Collect existing quick, then run one stable committed-source all; retain TK1–TK4 boundaries. |

## Implementation

First substantive portable implementation of **cSC4AnimationTickManager**, from
`docs/specs/animation-tick-manager.md` §§2–6,8–9. This is a new module and additive
Rust lib export; the different animation coordinator and all consumers are unchanged.
Implementation milestone cb77c6f3; independent tests and formatting e2bef109.

The core owns the four exact-u32 pointer sets, per-band references, FIFO temporary
references, suppression, priorities, frame masks, bucket timestamps, clamping,
return selection, mutable message, pause nesting, manual/auto stepping, and typed
QI/reference state. A Host supplies raw unit-3 samples and primitive effects only.
DoMessage can synchronously call the manager's registration/priority/Find/time/
pause/step/message operations and recursive tick; no host-computed schedules or deltas.

Portable table choice: fixed193 slots, `(pointer>>2)%193`, ascending slots and
prepend chains. This supplies specified membership/ref behavior without importing
Rust hash iteration, a guessed load factor, prime progression or transfer order.
Even the low-load order test is explicitly a portable test, not a native golden.
Public SDK `SC4HashSet.h` provides no applicable owner growth/transfer contract.

## Primary slot map (17 slots)

“Supported” below means the published normal-call portable domain, not native ABI
or native execution. Valid live interfaces, successful allocation, external owner
lifetime, and non-reentrant reference effects are required throughout.

| Slot | Method | Portable status / boundary |
|---:|---|---|
| 0 | QueryInterface | Supported typed identities; failed IID preserves output; no native offsets/thunks. |
| 1 | AddRef | Supported wrapping u32 count. |
| 2 | Release | Partial TK3: normal count/destructor effects; final Release in delivery Unsupported. Destruction does not Shutdown or Release contained entries/message. |
| 3 | Init | Partial TK1: owned flag/timer, raw slots3→9, reusable message/cycle; external retention/scheduling unknown. Missing framework/allocation outside normal domain. |
| 4 | Shutdown | Partial TK1/TK3: cycle break, slots10→4 when framework present, cancel queued refs, release all four sets, retain published fields. Requires external owner and no destructive reentry. |
| 5 | AddTickableObject | Partial TK4 ordering; specified per-band membership/ref/queued requests implemented. |
| 6 | RemoveTickableObject | Supported one-occurrence sentinel and specific-miss fallback; queued priority discarded. |
| 7 | FindTickableObject | Supported live-set membership, no queued-view/ref effects. |
| 8 | SetNewPriority | Partial TK4 insertion order; specified temporary-ref/remove-one/add/drop and resultless surface implemented. |
| 9 | DoAnimationTick | Partial TK4 order / conditional numeric environment / TK3 exceptions. Normal algorithm implemented; recursion returns0 before any effects. |
| 10 | GetTickTimestamp | Supported stored raw/shared u32, not rate. |
| 11 | SetMinAndMaxTickTimeDeltas | Supported unsigned normalization/clamp order. |
| 12 | PauseAnimationSystem | Supported wrapping bits, old signed-depth restart condition. |
| 13 | IsAnimationSystemPaused | Supported signed>0, distinct from tick's !=0. |
| 14 | EnableAutoStep | Supported interval store, transition Restart/Reset; repeat true does not Restart. |
| 15 | SetStepTickTimeDelta | Supported all raw u32 values. |
| 16 | StepAnimationSystem | Supported first-call pause-only, pending coalescing and sweep-end clear. |

Auxiliary persistence is **Unsupported (TK2)**: no guessed version1 schema or loaded
references. Framework ownership/cadence (TK1), destructive/exceptional or AddRef/
Release reentry (TK3), and native growth ordering (TK4) remain unanswered. No rollback,
exception swallowing, safe callback destruction, factory layout or thunk ABI claim.
Shutdown's pending cancellation branch is implemented, but V19's conceptual seeded
queue is not treated as a demonstrated safe public call path.

Numeric domain explicitly selects nearest-even/24-bit operations using general x87
helpers. Signed-load/unsigned-correction/literal3A83126F/store gives V20 bits
40000000/42C80001/43FA0001. Integer-derived large-input goldens distinguish a
premature f32 cast (16777217→46831270). Other inherited PC/RC settings remain
conditional. Phase0 rate preserves wrap+3 and deliberately has no zero guard.

## Verification and remaining gate

- Independent V1–20 ledger: `engine/core/tests/animation_tick_manager_vectors.md`,
  written before module execution; these are twenty proposed spec vectors, not twenty native runs.
- Actual new test functions: **21 integration +1 seeded arithmetic unit**. Focused
  log shows core **3310 passed,0 failed** (includes the new unit), integration
  **21 passed,0 failed**, and no ignored tests. They cover phases/bands, ref/event
  traces, FIFO/suppression/recursion, clamp/wrap/step, pause/auto, result selection,
  mutable messages, QI/cycle/lifecycle boundaries and literal float bits.
- Baseline `qbuild all` original session62996 terminal **exit0**: Rust5353 passed,
  0 failed/ignored; clippy0 diagnostics; native217/217; ctest68/68. fc4verify1133.5s.
  It reported tree changes during the run, so it is baseline only. Original PID28140,
  created2026-10-23T02:56:18.209986Z; durable summary `build-runs/animation-baseline.json`.
- Focused `qbuild quick` original session70309, PID13056 created
  2026-10-23T03:20:20.565438Z. Rust results above and incremental native4/4 are logged,
  but **the terminal result has not been collected**. Driver records:
  `build-runs/animation-focused.json` / `.log`; native/Rust details in
  `fc4verify-quick-test.log`, `fc4verify-quick-deps.log`, `fc4verify-build.log`.
- No overlapping checker, RPC, kill or restart. Baseline's delay was observed in its
  stats child; final success must come from the actual checker terminal, not log inference.
- New module/tests `qbuild fmt` **exit0**. Shared lib check reports existing module-list
  ordering diffs, also reproduced from baseline lib via stdin; no mass reformat.
  Initial relative-path fmt failed lookup; absolute worktree paths are required.
  q.py's Bash fallback was denied (durable q/20261023-041710-31284.log); PowerShell
  performed the read-only baseline comparison instead.
- Final stable committed-source canonical **all is not yet run**. This lane is WIP,
  not a final gate PASS. Source engine tree85a62cdf7acf9604653dcabb12ee284bf76b45e4;
  focused driver engine_before SHA256
  `7b83e4ad490a677f1653eb4d9bdfce5ea44f3be7d7ae5d66ea41403b0ef1d762`.
  Require the driver's terminal exit and matching engine_before/engine_after.

## Questions, scope and next steps

Appended **TK4** to `docs/specs/questions/animation-tick-manager.md`; TK1–TK3 retained
unanswered. The coordinator regenerates the question index; this lane edits no other spec.
No game requests, native execution, shadows, whole-owner/body-byte credit, default
promotion, C ABI exports, SDK, CMake, plugin, consumers or coordinator changes.
The 44-body/4286-byte and 234-byte helper inventories are attribution, not coverage.

Next: collect **the existing** focused checker result first (do not start another
quick/all while it is active). If it exits cleanly, run exactly one canonical final
`python build-runs/check-animation.py all animation-final` on this committed source.
That foreground driver records the exact owned child PID/creation/output, terminal
exit and before/after tracked-engine SHA256. Sum every final Rust test-result row,
record clippy/native/ctest totals, then update this report and queue to ready only
when the actual terminal and stable source hashes establish success.
