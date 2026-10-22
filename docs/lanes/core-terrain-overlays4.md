| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-terrain-overlays4 (Rust overlay state subset) | `core-terrain-overlays4` / engine lane | WIP; Git metadata deny-write ACL blocks commit; cargo4198/0 incl25 new tests, clippy clean, ctest67/67; OV4-OV9 filed; no ABI/runtime integration | (1) Restore metadata write access and commit preserved changes. (2) Resolve placement/model/render contract gaps. |

## Result

**Partial owner only. No whole-owner implementation, replacement, shadow, verified-game or family coverage credit is claimed.** The scoped public spec leaves the placement record and add arguments open under OV1, shadow/model construction under OV2, and message behavior under OV3. No further public behavior was available in the permitted source. This lane adds questions OV4-OV9 for precise additional blockers instead of reconstructing missing behavior.

Changes are confined to `engine/core/src/terrain_overlays/`, one additive `lib.rs` registration, the permitted question-file append, and this report. The new module owns its records, resource references, mutations and internal cell logic. External traits expose texture resources, terrain dimensions and view configuration/subscriptions only. There are no callbacks accepting original overlay results, placement matrices, clipping rectangles, circle parts, model footprints or successful render/counter outputs.

## Exact coverage and remaining work

| Method/helper | Implemented behavior | Remaining contract/work |
|---|---|---|
| Constructor / Init / shutdown | Rust state; acquire owned Arc references to view/factory/terrain, dimensions/scale/metres, dynamic/water/settings, static-index allocation, subscription; unsubscribe/drop references and clear records/free lists on shutdown and Drop | Original defaults, repeat/failing Init policy, shadow-binding factory slot and actual GPU draw-state initialization: OV4. Rust Arc ownership models owned references; no GZCOM QueryInterface/AddRef/Release adapter. |
| Slots3/4 source/binding decal | Null binding returns -1; failure retains no texture/source reference | Placement, size/margin equations, original argument meanings and record construction: OV1. Non-null construction explicitly errors. |
| Slots5/6 key decal | Distinct factory-slot6/slot5 lookups; absent binding returns -1 | Final slot4 argument forwarding and placement: OV1; non-null result errors, no allocation. |
| Slot7 round decal | Explicit OV9 error | Eight-part geometry, half-chord sampling/rounding, clipping, mapping, allocation and redisplay: OV1/OV9. No disc approximation or host-generated parts. |
| Slot8 city decal | Explicit OV2 error | Complete arguments/defaults/matrix and flags0x12 construction: OV2. |
| Slots9/10/11 shadow/model adds | Quality-zero slot10 gate returns -1 before any factory lookup | Complete signatures and shadow/model-footprint equations: OV1/OV2. No invented model/footprint API; `check_shadow_add` is a diagnostic gate, not claimed implementations of these slots. |
| Slot12 SetPosition | None ignored, direct overlay ignored per stated handle-record restriction | Nine-float x/z offsets and re-placement: OV1/OV5; indirect form errors. |
| Slot13 SetPlacement | None ignored; non-null fails atomically | Direct-handle behavior and placement equations: OV1/OV5. |
| Slot14 GetPlacement | Copies all nine opaque floats from an indirect record; none leaves output unchanged | Direct-overlay behavior: OV5. |
| Slot15 Remove | Removes group parts, owned texture/source refs, cell memberships, group and indirect records; frees slots | Original free-list order and invalid/stale policy: OV5; LIFO and prevalidation are explicitly core choices. |
| Slot16 SetShown | Mutates bit0 across direct/indirect/group records, records changed rectangles | Original equal-input invalidation policy: OV5. |
| Slot17 SetAlpha | Dynamic values and static0/1 endpoint changes; static intermediate changes fail without mutation | Exact step value/precision, comparison/history/storage behavior, clamping/NaNs: OV6. |
| Slots18/19 SetColor/SetDrawState | Mutate RGB/state byte across direct/indirect/groups and record changes | Original equal-input invalidation policy: OV5. |
| Slot20 EnableSelfLighting | Corrected odd-input OR bit6, even-input preserves bit6; resolves indirect and all eight group parts; -1 ignored; no redraw | Public corrected behavior covered; no whole-owner credit. |
| Slot21 SetTextureScale | Semantic scales1/size,-1/size and second-axis offset1+offset/size; bit5; groups | World matrix positions/conventions, zero-size policy and complete replacement of matrix: OV7. Semantic coefficients are not claimed as a completed world-to-texture matrix. |
| Slot22 getter | None ignored; non-null explicitly errors | Field identity/type and return policy: OV4. |
| Slots23-26 alpha/color/state/shown | Direct/indirect fields; alpha(-1)=0, other null outputs untouched | Scalar group result semantics: OV4; no arbitrary first/last-member selection. |
| Slot27 stats | Exact "%d decals, %d shadows" formatting | Real draw counter reset/increment points: OV8. Seeded format test does not claim3 decals/2 shadows were rendered. |
| Cell helpers | Own half-open cell clipping/coverage/iteration, static-cell candidate deduplication, index removal | Original endpoint/margin and placement/index insertion/update contracts: OV1/OV8. Production add/placement is unavailable; tests seed private records/index only. |
| InitDrawStates | All five semantic state descriptions, including state4 preserving stage0 mode | Packed GPU representation and actual GPU setup: OV8. |
| Draw/DrawDecals/DrawShadows/DrawShadowsRough/DrawRect | Static-visible versus dynamic-all decal selection, shown/part/snapshot gating, rough/full quality selection; dynamic-snapshot no-draw path | Camera inverse/composition, rectangle arrays, shadow classification/strength/height/combiner, GPU save/restore, actual drawing and counters: OV7/OV8. Normal Draw explicitly errors; selection is diagnostics only. |
| Change message0xC9F775BB | Subscribe/unsubscribe and report recognized-message gap | Actual dispatch/re-placement behavior: OV3. |

The new public APIs are portable core APIs, not claims about missing binary signatures. No guessed placement or model-footprint math was implemented and none was delegated. The specification must publish those owner equations before implementation can proceed.

## Validation

- Baseline `qbuild all`: PASS workspace tests (core2316), clippy clean, native ctest67/67. Logs preserved in `build-runs/baseline/`.
- `qbuild quick`: PASS core2341/0, dependent crates compile-only, native build4/4. Exactly25 new embedded Rust spec/ownership/failure scenarios; no language-neutral vector-runner edits outside the allowed module scope.
- Final `qbuild all`: PASS cargo4198/0 (core2341/0), clippy exit0/diagnostics0, native build2/2, ctest67/67. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- `qbuild fmt` with absolute paths: all four new module files PASS. Including touched `lib.rs` exposes existing module-order/wrapping changes; left the shared file additive rather than mass-reformat. No generated ABI file edited.
- Ownership/failure tests cover acquired-reference counts, unsubscribe-once/drop/re-init, failed Init rollback, failed adds releasing temporaries, removal releasing only owned texture/source references, and stale/malformed records failing before mutation. Send+Sync/getters have no RefCell.
- Spec scenarios include all eight group parts, corrected slot20 0/1/0 and low-bit2/3 inputs, indirect handles, no redraw, null calls, quality0 gate, texture-scale coefficients, dynamic snapshots and stats formatting. State fixtures are explicitly private test seeds, not substitute same-owner providers or conformance for missing add/geometry/render logic.
- No game request or shadow comparison: the change has no plugin/CAPI registration and cannot be exercised by the game. No runtime replacement credit; ABI unchanged.

## Questions and recovery

Appended OV4-OV9 to `docs/specs/questions/overlay-manager.md`; OV1-OV3 remain open (OV2's slot20 portion is superseded by the corrected public spec). Did not run forbidden tools or edit the generated question index; coordinator can regenerate it.

Git add/commit failed with `Unable to create .../.git/worktrees/core-terrain-overlays4/index.lock: Permission denied`. Read-only ACL inspection confirmed an explicit sandbox SID deny-write on that directory and index. No ACL modification, escalation or sandbox workaround attempted. **There are no lane commits; HEAD is801cbe48.** The worktree preserves tracked modifications and untracked `terrain_overlays/` plus this lane file. The intended UTF-8 commit message is `build-runs/msg-core-terrain-overlays4-1.txt`.

Coordinator recovery: restore permitted Git metadata write access, stage the four module files, `engine/core/src/lib.rs`, this report and the question append, then commit with the saved message. Retain WIP status until the listed missing contracts/methods are resolved. No merge/push/restart, game request or ABI change was performed.


### Coordinator delivery recovery (2026-10-22)
The worker stopped after Git metadata writes were denied; its original result and recovery evidence are retained. Coordinator reviewed and committed only the reported engine/docs files after the worker exited. The documented gate remains the worker measurement; the integration train must independently gate/regress this commit. Whole-owner and runtime credit remain withheld wherever the report lists missing contracts or bodies. No new lane round was used.
