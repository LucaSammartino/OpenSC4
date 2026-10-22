| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-terrain-ripples4: terrain ripples and selection | Codex / `core-terrain-ripples4` (engine lane, detached HEAD) | WIP: kernels committed 3fbf4788; 37 new tests; workspace cargo 4314/0, clippy 0 diagnostics, ctest 67/67; WR4–WR10; ABI unchanged; no game/shadow/complete-owner credit | 1. Answer WR4–WR10. 2. Complete individual owners against published method inventories, then validate. |

## Result

Implemented the published kernels of water-ripples §§1–2,6 in
`engine/core/src/terrain_ripples/`. Neither individual native owner is complete.
The spec does not publish enough information to implement tracing, textured
masks, rendering or the advertised 65-function inventory. The public
`OWNER_CHECKLIST` in `mod.rs` identifies each completed kernel and each blocker;
there is no owner-completion, shadow, replacement or runtime credit.

## Changes

- `tuning.rs`: exact exemplar/property routing, five zooms, shipped tuning,
  minimum shore length, spacing step, shared edit threshold and raw period range.
  Missing properties fail explicitly rather than silently becoming shipped values.
- `geometry.rs`: uniform cubic B-spline and analytic unit tangent; literal normal
  `(tz,-tx)`; inner/outer height and wobble; local pair spacing/turn/edge/1000-pair
  filters; short-strip rejection and even vertex counts; texture advance and fade
  pair count. Full traversal and alpha preparation are explicitly incomplete.
- Texture slot 6 uses the exact three-part ripple resource key. Failed lookup
  yields no bound strip; invalid/empty textures release temporary references.
  Arc owns the retained binding. This is a geometric/animation kernel, not a
  render-ready native strip or a host-provided owner result.
- Exact-boundary animation advances/wraps frames and resamples the raw period.
  Overshoot returns `AnimationOvershootWr6` atomically, because remainder/catch-up
  semantics are unpublished. No conversion of 175000 or 2000000 raw units.
- `ripple.rs`: exact opaque flag predicate and seed enumeration, remembered edit
  counter, immediate/delayed rebuild state, rectangle area/dirty accumulation,
  message IDs and empty-shore owner path. Any eligible seed stops detection with
  `ShoreTraceWr4`; terrain providers supply data, never traced shores.
- `selection.rs`: both vertex-indexed exclusive tables; all five marking forms;
  inclusive rectangles; clear behavior; five known palette entries; opaque bytes;
  packed RGB and mode-3/no-add-signed fallback. Unknown packed third byte is
  represented as `None`, never invented. Nonempty drawing returns
  `SelectionGeometryWr7`; four material states and masks remain unimplemented.
- Explicit shutdown and Rust Drop release only owned Arc references. State
  getters are Send + Sync with no RefCell. Native lifecycle compatibility is
  still blocked by WR8. Only one additive module entry touches shared `lib.rs`.

## Verification

- Baseline: accepted the coordinator's stable native train baseline; the task
  explicitly prohibited repeating a full pre-edit gate.
- Focused `qbuild quick`: PASS, core **2447 passed / 0 failed**, dependents
  `fc4-capi,fc4-conformance,fc4-formats` compile, native build **206/206** edges.
- One stable final `qbuild all` at implementation commit **3fbf4788**: PASS,
  workspace **4314 passed / 0 failed / 0 ignored**, clippy **0 diagnostics**,
  native build **2/2** edges, full ctest **67/67** (99.1 s).
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-quick-test.log`, `fc4verify-quick-deps.log`, `fc4verify-build.log`,
  `fc4verify-ctest.log`. Gate output was judged from wrapper PASS/count lines.
- 37 new Rust tests include the eight independently prescribed §6 vectors,
  asymmetric B-spline position/derivative and normals/wobble, exact raw tick and
  edit thresholds, full flag-byte combinations, vertex strides/last edges,
  selection exclusivity/clear/colour, invalid-input atomicity, texture failure,
  Arc lifetimes, pre-Init/shutdown, and concurrent read-only getters. Provisional
  local rules are tagged with questions; tests grant no native compatibility credit.
- `qbuild fmt`: six new Rust files PASS. Shared `lib.rs` check reports 125 lines
  of module-order differences, including pre-existing commands/cmdline ordering
  verified in the parent. Kept the required single additive module registration;
  no mass reformat. `git diff --check` PASS.
- No game requests made or outstanding; game verdict/ref_warnings/shadow n/a.
  No C ABI/plugin/maps/generated ABI-entry edits; ABI version unchanged.

## Questions and next steps

Appended **WR4–WR10** to `docs/specs/questions/water-ripples.md`; did not edit
the source spec, question index, or any analysis-side material.

1. **WR4:** ordered trace connectivity, priority, flags, endpoints and duplicates.
2. **WR5:** full spline walk, strip splits/retry, UV/order and exact alpha ramps.
3. **WR6:** random sampling/rounding, initial clock, overshoot and catch-up.
4. **WR7:** geometry, literal byte masks, normals/UVs, four selection states,
   ripple blend/frustum/depth state, and native tile submission order.
5. **WR8:** complete per-owner function/signature inventory, defaults, failure
   and acquisition/cleanup contracts; needed before either owner can be complete.
6. **WR9:** packed b3, alpha/mode widths, invalid inputs and overflow palette.
7. **WR10:** edit rectangle and counter semantics, tick arming/reset boundaries,
   dirty cleanup and immediate/zoom rebuild scope.

After these contracts are published, finish owners individually and expand the
independent vectors. The current WIP is reviewable kernel work, not a completed
replacement. Commits: b3b07171 initial handoff; 3fbf4788 implementation/questions.
