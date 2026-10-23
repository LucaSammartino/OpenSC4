| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-animated-sprites-contracts4: qualified setters and cached draw | Codex / core-animated-sprites-contracts4 | ready | Land bounded algorithms; retain explicit arithmetic/orientation Unsupported domains and residual owner questions. |

## Result

Implemented new settled public §§4.4/4.1/5.1/5.2/6 algorithms in the Rust
animated-sprite module. No C ABI exports, plugin, defaults, maps, serialized
schema, native instance layout or game verification changed. Worktree reused
at detached base 15aced92; no branch/ref changes, merges or pushes.

Source milestones: 704fd2e7 (qualified setters/cache and independent vectors),
e7bfb566 (raw second-triangle order and bounded fade-reference arithmetic).
The final full gate tested e7bfb566 with no source changes during the run.

## Implemented behavior

- Fade now implements off/reset, -100 retention, -200 reset, exact threshold
  bits 3F7D70A4, negative acceptance and completing quiet-NaN acceptance, with
  return 2. Highlight compares full integer against the unsigned old byte,
  stores low byte, and preserves full-input nonzeroness. Raw-byte self-lit
  compares against normalized flag 4; the ordinary bool adapter remains.
  Returns and unrelated state preservation are independently asserted.
- Exact-stage UpdateView owns d/t/h/B, corner signs/order and world-Y-only
  vertical edges. It resets the box every call, includes origin/v0/v3 and
  reflected prior maximum, omits lower corners, and can shrink. It uses
  already decoded/scaled metrics and already rounded foreign volume endpoints.
  Owner rotation/scale do not enter corner construction.
- Draw owns full context integer versus cached-byte comparisons, low-byte
  holder indexing, dirty refresh, view-only geometry refresh, byte-counter
  translation/staleness and ordered raw context calls. Incoming highlight
  pass or signed zoom below 2 returns 1 early; stored highlight does not.
  False foreign returns are ignored; only the two published resets occur.
- Raw eight-byte decoder owns valid temporal packing, page rollover, signed
  dimension flips/offsets, exact UVs and qualified scale. Refresh does not
  call the inherited provisional direction-major selector.
- Pick owns stale translated vertices, actual raw triangle order 012 then
  230, first-hit short circuit and the second primitive's boolean.

## Explicit limits

Geometry/translation/fade-reference arithmetic admits only exact stages with
nonzero operand/result magnitudes in [2^-12,2^12]. A f64 carrier proves exact
f32 sums/products; division is checked by an exact product equality.
This conservative subset respects the published native stores because no
admitted stage rounds. Inexact, larger, tiny, overflow and nonfinite regions
return Unsupported; there is no blanket f32/f64 or x87 PC/RC promise.
Signed-zero payloads are not claimed.

Dirty selection supports raw divisors [1,1,1], angle limits [binary32 pi,
binary32 pi,0,0], owner back (0,0,1), and low rotation 0. Then P=R=1 and both
B conversions receive zero; core chooses entry zero and owns its decoder.
Other orientation/holder domains explicitly return Unsupported. Clean cache
draws do not run selection. Invalid holders, malformed packing and missing
textures are checked portable errors, not successful empty draws or native
fault behavior. Explicit cache installation/stored-metric UpdateView are
portable current-state suboperations, not native factory/default/full-entry
proof. Existing transform/orientation choices are not promoted.

AS8 is implemented only in its completing-comparison domain. AS1/2/3/4/5/6/7/9
whole-call, control-word, trig, resource, controller, defaults, sound and
lifetime unknowns remain. The 60-function/10482-byte catalogue remains
unreconciled. R2-R ideal rotations remain qualified geometric UNRUN; the 72
R3 conditional raw-stage examples are not angle-derived native goldens and
receive no owner-execution credit. No full owner/native/default/map credit.

## Validation

Expectations were transcribed as literals before new production outputs.
Eight new portable contract test functions cover setter sentinels and
threshold-adjacent bits, raw highlight/self-lit inputs, unrelated state/cache
sentinels, Q1–Q5 corners/bounds, shrink, returned-volume fallback, mpp/camera
staleness, dirty/frame-only transitions, counter equality/wrap, repeated full
zoom 258/rotation 256, raw protocol/ignored false returns and stale Pick.
The public packed-entry/frame-5/scale-2 decoder vector is also literal.
No expected value comes from a production helper, native oracle or case ID.
All 37 current animated-sprite test functions passed (29 inherited + 8 new).

Focused quick completed with 3409 passed/1 failed; dependents compiled and
native build completed 217/217 edges. Its independent Pick test exposed the
inherited second primitive argument order 023. Corrected production to 230
and changed that contradictory old expectation with an explicit public
DIFFERS comment. Fade/highlight historical contradictions were likewise
changed only where §§4.4/5.2 explicitly supersede them. Quick reported a
changing tree and was not used as the final gate.

Final format wrapper: exit 0, five owned Rust files (absolute paths).
Initial relative-path format invocation failed to resolve files; corrected
without adding unrelated formatting. The canonicalization warning was the
only final formatter message.

ONE stable-source qbuild all completed, exit 0:

| Check | Measured result |
|---|---|
| cargo test --workspace | 5507 passed, 0 failed, 0 ignored; 26 result summaries |
| clippy -D warnings | exit 0, diagnostics 0; 41.61 s |
| Native CMake build | exit 0, PASS 4/4 edges; 84.7 s |
| Full CTest | exit 0, PASS 68/68; 183.1 s wrapper / 182.81 s CTest |
| fc4verify | PASS, 403.4 s |

All 26 Rust result-summary passed counts, in log order:
540,3,1,31,27,40,7,2,1232,3410,21,13,19,147,0,2,1,3,1,1,1,5,0,0,0,0.
Every failed and ignored count is zero. Full logs are retained at
build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log and
fc4verify-ctest.log. Durable collected terminal:
build-runs/qbuild-all-terminal.txt. Historical 29/4306/67 counts are not the
current gate.

Engine proof before/after the same all operation: MATCH.
Source HEAD e7bfb56621f2d7d18d0085908c11f74d791c2a9c;
engine tree b9a8d51a8ca51188e852f2c20e9cf6ba23d73f8e;
tracked engine-content SHA256
1ec44d3328973063ad5a2357eea339ae021d9e5ac6f0d6a3b8bd8e324f7e6d83.
Proof snapshots/helpers remain in this worktree's build-runs. Only this
lane report changes after the verified source.

## Questions and next steps

New questions: none; the remaining unimplemented domains are covered by
existing AS questions and explicit Unsupported results. No game request was
made or remains outstanding; native owner execution/shadow is UNEXECUTED,
as required by this Rust-only task. All asynchronous gate sessions completed
and their terminal results were collected. No cancellation, duplicate full
gate, server restart or configuration change was used.

Ready to land this bounded implementation. Future work can broaden qualified
selection/arithmetic domains from public settled contracts and reconcile the
owner catalogue; it must not infer native trig/default/lifetime behavior from
these portable tests.
