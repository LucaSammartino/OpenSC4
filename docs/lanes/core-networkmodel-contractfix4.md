| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-networkmodel-contractfix4 (published network model corrections) | `core-networkmodel-contractfix4` / detached reserved worktree (engine lane) | **wip**: code 45f9df89, 847216a1; final cargo 4184/0/0, clippy clean, ctest 67/67, fmt PASS; NO27-NO31; ABI unchanged; game n/a; whole owner incomplete | Coordinator review scoped corrections and exact remainder below; preserve family cap 3 |

## Changes

Coordinator source-publication review (2026-10-22): accept the committed published-contract corrections for merge-train integration. Review confirms the removal of same-owner output callbacks, explicit unsupported mode-0 cut/lighting results, retained resource/cache ownership and discriminating tests; the clean stable gate is source evidence only. NO27-NO31, allocation/precision limits and the family cap remain. Preserve the parked worktree; no full-owner/native/default credit or extra round is authorized. Gate the actual merged source.

Applied the published NO21-NO23/NO25/NO26 contracts from network-occupants.md §§10.1-10.3 in the Rust core. No plugin, CAPI, ABI, tools, generated ABI-entry, or game-file changes. The four model bodies are ordinary members, not virtual overrides; the obsolete ModelRenderer owner-result callbacks and bilinear/frame assumptions are removed.

- Deform uses the published two planes, centered ±8 coordinates, unclamped extrapolation, rotation then flip then path-origin subtraction, piece-relative Y and additive flat-model relief. A separate terrain-height constructor applies height + 0.2 before plane generation; draw state supplies actual quad heights directly.
- Bind retains the shared source arrays and makes private position/UV copies with an explicit processor colour/format configuration. FlipAndLight releases those processor source references. There are no default native constants.
- Flip negates transform row 0 and reverses/synthesizes indices with primitive-range rebasing. It does not mirror vertex coordinates or UVs. The lighting request negates all nine rotation entries when flipped.
- Clip implements the published null/contained/unordered early-outs, inclusive y slab, all published source formats and unsupported-format empty output, triangle-list ranges, fan winding, source-index sharing and unshared cuts across input triangles. Fan vertices within one polygon follow worked vector A. Cuts use outside-anchored f32 t, widened delta arithmetic and colour-byte truncation; bounds clamp on every axis instead of recomputing. Mode 0's published x/z early-out is implemented; actual six-plane cutting remains an explicit typed remainder.
- Plain queries always return the actual embedded instance and update its translation/mask/change counter. Prebuilt queries construct fresh heap instances, fill LOD-0 meshes/bounds from external resources, apply the pass/transform/alpha sequence and retain caller + cache references. Load failure preserves the old cache; SetModel failure clears it. Zero main ID skips second-model work and shadow update. Main IDs remain at the external resource boundary; only the second model uses the published fixed TGI.
- Active Shutdown clears main then second cache; inactive Shutdown touches neither. Destruction releases second then main. Caller references survive replacement/shutdown. Embedded transform getters use RwLock, and model/owner types are Send + Sync. No RefCell, renderer/cache callbacks or original owner results are used as host inputs.

## Owner inventory and exact remainder

Whole-owner completion is **not claimed**. The inventory is:

- `Bind` / former Prelight (`0x00614350`): copy, bounds preservation and source-reference lifetime implemented. Numeric colour, packed source format and processor destination format require NO27; tests pass explicit fixture constants rather than guessed native values.
- `Deform` (`0x00614A90`): published plane/rotation/flip/origin/Y/bounds algorithm implemented. Arithmetic uses f64 intermediates and explicit f32 stores; this is not a full 80-bit x87 emulator or an exhaustive precision-equivalence proof.
- `FlipAndLight` (`0x00616800`): winding, transform convention and ReleaseModel reference effects implemented. Processor normal generation and final lit colours remain a pending LightingRequest (NO28), not a successful lighting result.
- `Clip` (`0x00618B00`): network mode 1 and both modes' published early-outs implemented. Non-null mode-0 calls needing a cut return UnspecifiedClipMode without fabricating geometry (NO29).
- Construction (`0x00614A30`), SetModel (`0x00614890`) and destruction (`0x00614830`): fresh instance/resource/mesh ownership and LOD/bounds modeled. Unpublished constructor/destructor body effects remain NO31. Stable safe Arc allocation aborts on OOM; the native null-new branch that clears the old cache is **not implemented**.
- The §10.1 caller and cache release paths are modeled, including primary-slot-81 dispatch and UpdateShadow as returned effects. This is not a replacement of the complete prebuilt subclass, its appendix body, base Shutdown or shadow-generation body. The clipped/skip-option ambiguity uses the literal exception for Clip alone and remains NO30.

## Measured verification

- Base: `94eeb430`. Required initial `qbuild all`: workspace cargo/clippy PASS, native ctest **67/67**.
- Code commits: `45f9df89` (geometry, ownership, first vectors), `847216a1` (resource/exception audit and further vectors).
- Quick: core **2324 passed, 0 failed**, dependents compiled, native build **4/4** edges; PASS. Intermediate full: **4184/0/0**, clippy clean, ctest **67/67**; source changed during that run, so it was repeated.
- Final `qbuild all` on clean committed code at `908554c3`: workspace **4184 passed, 0 failed, 0 ignored**; core **2327/2327**; clippy **0 diagnostics**; native build **4/4**, ctest **67/67** in **131.9s**; fc4verify **PASS (221.8s)**, with no tree-change annotation.
- Required `qbuild fmt` with absolute paths: **exit 0, 3 files**. The canonical-path warning is environmental; no formatting diff. The first relative-path invocation failed to locate files and was corrected.
- **17 model tests** discriminate both diagonals/midpoints/extrapolation, additive relief and height + 0.2, rotation/flip/origin order, private Bind copies, indexed/nonindexed winding, NO25 A-D (D x bits **0x3F800000**, colour truncation **0x010203A9**), both slab planes, source sharing/nonshared cuts, range rewrites, all-axis clamping, unknown/empty formats, NaNs, no-model and warm zero-ID cases, independent cache failures, actual reference counts, fresh draws, shutdown/destructor release order and cross-thread getter types.
- Logs: `build-runs/qbuild-{test,clippy}.log`, `build-runs/fc4verify-{build,ctest}.log`; intermediate logs preserved under `build-runs/intermediate-full/`.
- Game: **n/a**. This pure-core API has no plugin/CAPI observation path; no game request, verdict, shadow-exact or ref-warning claim, and no outstanding request.

## Questions and next steps

Appended **NO27-NO31** to `docs/specs/questions/network-occupants.md`: processor constants/formats; processor lighting; six-plane clipping; clipped-option ordering; instance constructor/destructor effects. Only that allowed spec question file was edited; the coordinator can regenerate the question index.

Review the scoped correction commits, obtain those contracts, and address the explicit allocation/precision remainders before claiming whole-owner completion or wiring a runtime replacement. Queue state is **wip**, ABI false. This round launched no child worker, restart, game, merge or push; retain the conservative **family cap 3**, with no fourth round requested.
