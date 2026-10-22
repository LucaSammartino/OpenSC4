| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-glyph-textures4: glyph and string texture portable owners | Codex / `core-glyph-textures4` (engine lane, detached worktree) | ready for code review; source 6c9c6f99; cargo 4308/4308, clippy 0 diagnostics, ctest 67/67; GT3–GT6 filed; no ABI change or complete-owner promotion | 1. Resolve GT3–GT6 and reconcile the 43-function inventory before complete-owner credit. 2. Native binding/shadow is a separate task. |

## Result

Added `engine/core/src/glyph_textures/` with the portable state and algorithms expressly published in `glyph-textures.md` §§1–3 and the §7 scenarios. The only shared source edit appends the module declaration to `engine/core/src/lib.rs`. No CAPI, plugin, ABI, coverage map, generated ABI entries or native bindings changed.

The renderer, font lookup, framework string comparison/character lengths and framework registration are explicit foreign primitive services. The module owns key ordering, sharing, reference decisions, dimensions/UV calculations, cache identity and order, second-chance eviction and lifecycle ordering. Font measurement reuses the existing `fonts::FontRef` through an acquiring/releasing `CoreFont` adapter. Binding/font values are owned leases, so dropping them releases only references acquired or transferred by their providers. There is no RefCell; the host can synchronize this exclusively mutated owner with a mutex and use shared getters safely.

Safe typed tokens replace pointers. Token copies do not AddRef. Native AddRef/Release counts remain explicit, including counted manager links, map/cache references and the string count-1-without-manager destruction quirk. Invalid/stale tokens return failure rather than accessing freed storage. Whole-context Rust Drop is host teardown, not a claimed native deleting-destructor flags contract.

## Covered and missing checklist

| Owner | Implemented published bodies/paths | Missing or provisional contracts; credit boundary |
|---|---|---|
| `cSC4GlyphTextureManager` | IID-only QI, AddRef/Release; Init once with KB budget and registration/subscription order; Shutdown detaches/releases glyphs and leaves strings cached; sorted glyph lookup, sharing and failure; code-page glyph-array append with failed characters skipped; string object creation and lazy per-object cache; process-wide miss counter; hit-to-newest ordering; strict >50ms sweep, rollback, target and one-pass second chance; restore clears strings then replaces glyph bindings in map order. | Constructor count is explicit caller input; zero initial sweep time and shutdown-then-cache context cleanup are provisional. Exact constructor/destructor and deleting flags, inherited/shared service and auxiliary slots, message-target thunks and the complete 43-body inventory require GT4. No complete-manager credit. |
| `cSC4GlyphTextureInfo` | Published slots 1–8: map/caller references, last-external-reference removal, character copy replacement, u16 sizes exposed as u32, UV normalization, binding observation, counted manager detach and manager-before-binding destruction. | Native slot-0/inventory and binding getter acquisition are not published (GT4/GT6). Boundary arithmetic and restore metadata behavior remain literal choices under GT3/GT5. No unqualified native complete-owner credit. |
| `cSC4StringTextureInfo` | Published slots 1–5: retained manager/text/font, padded power-of-two size, explicit AddRef/Release including manager-less count-1 deletion, lazy texture handle through cache, and font/text/manager destruction order. | Omitted constructor/slot-0 inventory requires GT4; null-font and failed-output contracts require GT6; extreme dimensions/accounting require GT3. No unqualified native complete-owner credit. |

The public ABI audit found manager-labelled rows but did not reconcile the brief's 43 functions/4475B to a fully published body list for all three owners. GT4 requests that exact inventory. No shared body was inferred from its unrelated ABI symbol name. This change is mergeable portable core work, not a claim of full 3-owner/43-function coverage, shadow or replacement.

## Verification

- The coordinator supplied the gated fresh-main baseline; no redundant pre-edit full gate ran.
- First `qbuild quick`: PASS, 2440 core tests, dependent crates compiled, CMake 206/206 edges. The wrapper noted a concurrent commit/docs metadata change; there were no concurrent source edits. This is supporting evidence, not the final stable gate.
- Final `qbuild all` on committed source **6c9c6f99**, with no edits during the run: **PASS**. Rust workspace **4308 passed / 0 failed / 0 ignored**; clippy **0 diagnostics** with `-D warnings`; CMake **4/4 edges**; full ctest **67/67**, 163.8 seconds. Final fc4verify **PASS**, 259.0 seconds.
- **19 focused glyph tests** pass. They exercise every published §7 scenario, fixed independent size/target vectors, case/font key ordering, code-page append/skip, reference/resource order, render/lookup failures, retries, restore with/without factory, strict tick boundaries, rollback/wrap and Send/Sync suitability.
- `qbuild fmt` passes on both new Rust sources. Checking all touched sources also reports **126 diff lines in shared `lib.rs`** (existing ordering/style plus relocation of the appended declaration). Its only source delta is the additive end-of-list declaration; the required no-mass-reformat rule was preserved.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`; initial focused evidence in `fc4verify-quick-test.log` and `fc4verify-quick-deps.log`.
- Real game: **n/a**, core-only task; no binding, shadow key or game request. No native equivalence/ref-warning verdict is claimed.

## Questions and current literal choices

Appended **GT3–GT6** to `docs/specs/questions/glyph-textures.md`; existing GT1/GT2 remain untouched. The coordinator must regenerate the question index (lane tools/source-side index generation is outside this core-only edit scope).

- **GT3:** widths/signedness, power-of-two overflow, narrowing versus UV division, cache-byte dimension source, arithmetic precision and zero budget. Core checks unrepresentable P, then assigns u16 fields; uses renderer dimensions for UVs, stored string dimensions for bytes, wrapping u32 fields and literal f64 sweep target/truncation. Overflow returns a typed unsupported-domain error, not a claimed native outcome.
- **GT4:** exact body inventory, constructor counts/time, destruction order/flags, inherited/auxiliary slots, manager link setter/removal behavior. Core takes the initial manager count explicitly; complete-owner credit remains withheld.
- **GT5:** restore metadata and failure. Core releases old binding before lookup/render, keeps dimensions/UVs, leaves binding empty on failure and retains glyphs untouched when no factory exists.
- **GT6:** binding acquisition, omitted QI/failure-output contracts, null string font, invalid character lengths and empty text. Core exposes a borrowed binding; string null font is a typed error; bad character lengths stop safely; empty text appends no glyphs.

## Next steps

Resolve these questions and reconcile every owner body before claiming complete-owner coverage. Then refine the identified literal edges and add discriminating vectors for the newly published contracts. Native glue and real-game shadow/replacement require a separate authorized lane. This round spawned no agents and performed no restart, merge, push or shared-ref update.

Commits: **b46c697a** scope/handoff; **d2abd604** substantive implementation and questions; **6c9c6f99** review, global counter, ordering and formatting. The final documentation commit records this report; queue tip is the actual detached HEAD.

## Coordinator source-phase review

Bounded portable code/findings publication only; GT3-GT6 remain. No complete-owner, shadow or replacement credit is granted for this partial phase. Preserve the parked original worktree and existing round budget. Any engine change requires the joint full native gate and current default-union regression.
