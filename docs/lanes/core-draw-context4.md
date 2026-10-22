| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-draw-context4: pure-core 3D draw context owner | `core-draw-context4` / (engine lane, detached reserved worktree) | blocked for owner promotion; code 5fee3804; cargo 4261/0, clippy 0 diagnostics, ctest 67/67; 23 new conformance tests; DC3-DC8; ABI unchanged; game n/a | 1. Publish missing contracts. 2. Complete affected bodies and reconcile questions. 3. Re-gate before owner promotion. |

## Outcome

The specified portable state/draw paths and 23 independent conformance tests are committed. The complete
11679-attributed-byte owner cannot be certified from the supplied contract: Begin/End are absent, both owned
combiner records contain omitted bytes, and native default stage-record values are unpublished. The lane is
blocked for promotion, with a `wip` queue record only. No completeness or replacement-coverage claim is made.

Changes are confined to `engine/core/src/draw_context/`, one appended `lib.rs` registration, this report, and
append-only questions in `docs/specs/questions/draw-context.md`. No plugin, CAPI, ABI, tool, generated ABI-entry,
game-file or shared configuration edits. The reserved checkout remains detached; the supervisor publishes its
commits to the reserved branch. No merge, push, self-restart, subagent or game request was made.

## Implemented paths and owner-method accounting

| Spec | Owner methods / core mapping |
|---|---|
| 1, 1.1 | Constructor (`new`), once-only `Init` (`init`), zero owned state, 11 highlight colours, default/highlight packed states, driver count and DC1 bytes, max(count, 2) stage storage. Combiner fragments and unknown default-cache fields are preserved explicitly. Renderer-populated scalars and unused borrowed renderer/camera-control identities are retained. |
| 2-3 | EnableColorMaskFlag, EnableDepthMaskFlag, EnableDepthTestFlag, EnableAlphaTestFlag, EnableBlendStateFlag, EnableCullFaceFlag, EnableTextureStateFlag; SetDepthFunc, SetAlphaFunc, SetBlendFunc; SetModelViewTransformChanged; SetTransparency and ResetTransparency. Cached word groups, keep sentinels, force switches, texture dependencies and suppressed stage-1 state are owned here. |
| 5 | PackRenderState (`PackedState::pack`), SetRenderState(packed), SetDefaultRenderState, SetDefaultRenderStateUnilaterally. Masked fields remain untouched, dispatch follows word-group order, and wrap/filter/transform resets occur after packed application. Core stack storage helpers are not claimed to reproduce unpublished native stack entry points. Highlight-colour setup is explicitly blocked by DC4. |
| 6 | SetRenderState(material, drawFlags) (`set_material_state`): flag-derived state, grey/fade effects, alpha quantization, depth overrides, slots and numeric texture transforms. Highlight-material setup is accounted for but blocked by DC4; default-combiner material paths fail before changing draw state. |
| 7 | SetTexture, SetTexCoord, SetTexWrapModes, SetTexFiltering, SetTexTransform4, ClearTexTransform, SetTexEnvModeForStage, SetTexCombiner, SetTexColor; wrap/filter overrides and suppressed-stage recording. Arbitrary caller combiner bytes are opaque; owned default/highlight bytes are never synthesized by a provider. |
| 7.1 | SetTexEnvMode (`prepare_second_pass`): texture/coordinate/wrap/filter/transform replay, equal depth, alpha-test disable and mode 0/1/2/other blend decisions. Previous transforms are cleared before restoring the recorded coordinate set so clear cannot overwrite that set with 0. Unknown defaults cannot become fabricated second-pass bindings. |
| 8 | SetCamera, both SetModelTransform overloads, SetDepthOffset, SetShade, SetSelfLitShade, ResetShade, SetModelShade. Projection transposition, column-major matrices, rotation/scale/translation composition, validity transitions, depth-bias multiplication, tint and shade multiplication are core computations. Camera matrices/depth bias and lighting queries are external interfaces only. |
| 9.1 | SetVertexBuffer, SetIndexBuffer, both DrawPrims forms (`draw_prims`, `draw_arrays`), both DrawPrimsIndexed forms (`draw_indexed`, `draw_indices`), RenderMesh. Vertex/group ordering and base + 2*first arithmetic are owned; addresses are synchronously forwarded without dereference. |
| 9.2 | RenderModelInstance: box path, highlight/bit-3 skip, threshold culling, instance transform, shading, per-mesh material/draw/pass order and alpha cleanup. Highlight drawing beyond early exits remains explicitly blocked. |
| 9.3 | DrawBoundingBox: six lit quads, published face/corner order, light (0.25, 0.5, 1), wrapping BGRA conversion using existing read-only x87 math, packed box state and draw commands. Geometry comes from geometry-utilities 6.1-6.2, not a same-owner host callback. |

External boundaries are typed driver commands and camera, lighting, view and instance interfaces. They do not
compute packed states, material decisions, transforms or model ordering. Rust shared resources retain owned
references; dropped contexts release those references without releasing borrowed native identities. Device
failure tests verify that failed fields do not advance the cache and completed prefixes remain available for
retry. Core getters contain no RefCell.

## Exact incomplete bodies and unresolved behavior

1. **Begin / End:** both return `Gap(BeginEnd)`; no driver/lifecycle algorithm or native signatures are published.
   Native shutdown/destructor reference semantics and a complete owner-method inventory also need DC3. Rust
   resource cleanup is implemented, but is not asserted to reproduce those unpublished native methods.
2. **SetRenderStateHighlight(colour):** the driver body is blocked by omitted highlight-combiner bytes. The
   material overload consequently fails at this call. **RenderModelInstance(highlight=true)** implements the
   specified skip/culling decisions, then rejects an actual highlight draw before modifying model/driver state.
3. **SetRenderState(material) mode 4:** loading the owned default combiner is blocked by its omitted bytes.
   Material/model preflight returns `Gap(DefaultCombiner)` before drawing. Explicit caller combiner records work.
   The 12-position fragment windows follow adjacent field offsets, not a certified driver-record wire size.
4. **Init and SetDefaultRenderStateUnilaterally exact stage defaults:** unknown coordinate/wrap/filter values,
   binding and transform knowledge are represented explicitly. First explicit writes establish known values;
   replay requiring unpublished values returns `Gap(StageDefaults)`. Exact native initial-cache suppression and
   default-record reconstruction remain incomplete until DC5. No missing byte value is fabricated.
5. **Reconciliation required before promotion:** invalid-input/provider failure channels and selection order
   (DC6), suppressed-stage wrap/filter disagreement with graphics-formats (DC7), and shade/default/matrix/rounding
   details (DC8). Current literal choices are documented in code and the questions, not asserted as measured game
   behavior. Unpublished native method coverage cannot be inferred from the ABI rows for other callers.

## Verification

- Initial `qbuild all`: cargo **4238 passed / 0 failed**, clippy **0 diagnostics**, CMake **206/206**, ctest
  **67/67**. The baseline run overlapped initial edits and is a baseline observation, not the final gate.
- Final `qbuild all` on committed code **5fee3804**, with no edits during the run: cargo **4261 passed / 0 failed /
  0 ignored**, clippy **0 diagnostics**, CMake **4/4** (49.4 s), ctest **67/67** (108.1 s); fc4verify **PASS**
  (159.5 s), qbuild **PASS**. The final report commit changes documentation only.
- All **23** new tests pass, including the nine draw-context section-13 cases plus independent expected-word,
  command-order, noncommuting rotation, projection, material fade/grey, box geometry, resource lifetime, failure
  cleanup, suppressed-stage replay, hardware-stage, unknown-default and mask/force tests.
- `qbuild fmt` with the absolute new module path: **PASS**, recursively checking its child modules. Checking all
  touched files also reports shared `lib.rs` formatting/reordering differences (126 diff lines). The required
  append-only registration remains a one-line diff; unrelated modules and existing formatting were not changed.
- Intermediate failures were corrected: draw-bit integer type at compile time; second-pass coordinate set 7
  being overwritten by transform clear. A gate overlapping the refinement failed cargo_test (66/67); it is
  superseded by the clean final gate above.
- Full logs remain in this worktree's `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`,
  `fc4verify-ctest.log` and `qbuild-fmt.log`.
- **Game / shadow / ref warnings: n/a, not measured.** There is no plugin/CAPI integration or shadow key for this
  core-only owner, and no ready promotion or game request. No verdict or ref-warning count is fabricated.

## Questions and next steps

Appended **DC3-DC8** to `docs/specs/questions/draw-context.md`: lifecycle/inventory; complete combiner records;
stage defaults; failure/selection behavior; suppressed-stage disagreement; shade and numeric details. DC1 driver
bytes remain exactly opaque and DC2 flags retain neutral bit labels. The generated QUESTIONS index is untouched
because this lane may edit only its spec question file; the coordinator must regenerate it using its authorized
workflow.

Spec writers must supply the missing contracts. Implement the remaining bodies and reconcile DC6-DC8, then run
the full gate again before owner promotion. Runtime adapter and shadow work can follow only after the core owner
is complete. There is no request outstanding and no restart/second round launched by this lane.

## Implementation commits

- **9635fa62**: first implementation and independent conformance, committed before call 30.
- **6b582745**: compile fix, additional transition/order/error/lifetime tests, owned combiner fragments.
- **5fee3804**: unknown stage knowledge and second-pass coordinate preservation; final gated code.

Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-draw-context4-r1-manifest.json`.
