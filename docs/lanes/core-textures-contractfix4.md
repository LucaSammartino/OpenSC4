| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-textures-contractfix4: texture/material contract corrections | Codex / `core-textures-contractfix4` (engine lane) | ready; final correction round 2/2, logical 3/3; source 399d555d unchanged; cargo 5098/0, clippy PASS, native 214/214, CTest 68/68; ABI unchanged | Coordinator review of bounded portable correction; retain TB1/TB2/TB8 limits and parent evidence. |

## Report

The existing production owner remains `engine/core/src/textures_materials/`; no second owner, S3D decoder, native adapter, default routing, ABI export, shared/generated table, glyph code or game file was added. No game requests were issued, as required by this brief. Original parent `core-textures-materials4` remains parked with its train84 evidence. This is final correction round **2/2**, logical family round **3/3**; no fourth round or renamed continuation is available.

The final gate passed on unchanged source **399d555d3f13e48eefcdd675921cdfa8159b6e20**. Implementation history remains in `e9ee879f`, `2a906396`, `f471bdc8` and `399d555d`; previous report `ca37f85f` and the corrected liveness audit `5a248e54` are preserved. This resume changed the lane report only; its supervision and evidence helpers remain ignored under this worktree's `build-runs/`.

### Changes and boundaries

- Signed power rounding returns 1 for words with bit 31 set, detects the native nonterminating positive interval, and never executes that loop. LOD subtraction, masked shifts, level/frame-zero calculations, byte products and aggregate counters now use the published u32 operations. The 65536-square case accounts zero bytes without allocating image storage.
- Binding references wrap at both ends. Only a decrement to 1 with a factory returns to the pool; a decrement to 0 destroys the binding. Material counts remain separate u16 words: acquisition wraps, while release at either 0 or 1 releases stage 1 then stage 0 and frees the material.
- Factory cache accepts instance zero. Reacquisition avoids load/upload; reuse deliberately preserves stale zero-key entries, and a later lookup can return a repurposed record. Eviction preserves the dangling token and reports an unsupported memory domain instead of pretending native lookup safely fails.
- Driver creation cleanup is one bulk deletion with exactly the number of earlier successful calls, including count zero and earlier zero handles. The current failed handle is excluded. Rust reclaims its temporary handle vector: the native allocation leak on this path is deliberately **not** claimed as parity.
- Upload reads the source description first, uses source frames and binding levels, wraps source-level subtraction, masks dimension shifts and forwards readable null pointer entries. Driver upload returns are ignored and no error query follows. Inaccessible tables and excess source frames explicitly stop at a portable boundary; they are not native false/error contracts.
- Restore visits every entry referencing one record, including repeated aliases; it reconstructs load keys from current record fields and releases each returned resource, including null-image resources. Traversal among distinct records is withheld under **TB8**, because native hash/growth/insertion state is unpublished. It makes no load/upload calls in that unsupported domain. This replaces the old sorted-key parity implication.
- Material manager exposes unnamed slots 6/7 as two-argument shared-false operations and destroys its foreign base without enumerating materials. Constructor count zero and creation acquisition are separate. Stage assignment acquires the new binding before storing/releasing the old; value assignment copies the count and header, then stages 0/1; range cleanup walks values forward and stages in reverse order.
- Safe tokens replace native allocations and pointers. `Unsupported` explicitly identifies native nontermination, unreadable image/handle tables, dangling keys, absent services, destroyed owners, invalid material/binding tokens, unsupported material slot counts, bounded allocation/work and unknown restore order. Convenience `Option` getters must be read with `unsupported()`; material creation has a checked `try_create_material` API. Raw constructor header placeholders and unknown byte +0x0B are not asserted as native initialized bytes.
- Caller-reference discipline is required for factory helpers. Releasing the factory's own last reference through the safe helper triggers record cleanup; the native history in which a binding allocation is freed at count zero while its factory record survives is outside this helper's supported ownership domain. The standalone binding's published decrement/return branch is implemented, but no parity is claimed for subsequent calls through freed pointers or that dangling-record history.

### Body and entry checklist

This is a reconciliation of **53 exclusive class bodies / 4749 bytes + 5 free functions / 134 bytes = 58 / 4883**, not 58 class methods or a blanket completion claim. Addresses below come only from the published spec/ABI tables. “Portable” means normalized Rust semantics for the supported memory domain, not a native ABI implementation. Exact pointer adjustment, object layout, native allocation/free behavior and hash topology are withheld throughout.

| Published body or entry | Portable implementation / withheld scope |
|---|---|
| Binding `004664A0` | `Binding::add_ref`, wrapping u32 |
| Binding `008090C0` | `Binding::new`, normalized texture-array/factory/reference storage; native allocation layout withheld |
| Binding `008090E0` | `Binding::release`, exact word branches; factory callback through `release_binding` |
| Material `00800D20` | `Material::constructor`, count zero and default stages; unspecified header values remain placeholders |
| Manager `00800C00` | `query_interface`, known IID and +4 foreign-base fallback |
| Manager `00800C50` | `stats_string`, Nothing to report |
| Manager `00800C70`, `00800CA0` | QI adjustors normalize to `query_interface`; native receiver arithmetic withheld |
| Manager `00800CD0` | normalized deleting-destructor dispatch |
| Manager `00800CE0` | `deleting_destructor`, foreign-base destruction, flags bit 0 free decision; no material enumeration |
| Manager `00800D70` | `try_create_material` / `create_material`; count acquisition and MATS projection; allocation failure/T>2 invalid-memory paths withheld |
| Factory `00809130` | `query_interface`, numeric identities/offsets only; TB1 semantic interface identity withheld |
| Factory `008091B0`, `008091C0` | `add_ref`, `release`, foreign system-service delegation |
| Factory `008091D0` | `shutdown`, flush/release/unsubscribe order |
| Factory `00809260` | `stats_string` / `Stats::format`, wrapped totals |
| Factory `008092F0` | `set_driver`, identity check, flush and owned references |
| Factory `00809330` | `BindingParams::for_image`, owner format selection |
| Factory `008093A0` | `init`, published acquisition/failure sequence |
| Factory `008094C0` | `get_binding_by_id`, hard-coded default group |
| Factory `008094F0` | `BindingParams::for_image`, signed power/masked LOD/wrapped dimensions |
| Factory `00809690` | key erasure in reuse, resource-group change and destruction; zero sentinel preserved |
| Factory `00809890` | normalized destructor / `Drop` / `shutdown`; native tables/allocation and exact table traversal withheld |
| Factory `00809910` | `set_lod_offset`; TB2 external producer withheld |
| Factory `00809920`, `00809930`, `00809940`, `00809950`, `00809960`, `00809970`, `00809980` | published adjustor entries normalize to QI/AddRef/Release; native pointer thunks withheld |
| Factory `00809990`, `008099A0`, `008099B0`, `008099C0`, `008099D0`, `008099E0` | remaining normalized reference/destructor adjustors; native pointer thunks withheld |
| Factory `008099F0` | `do_message`; single-record entry multiplicity, current-record key, failures/releases implemented; distinct-record traversal withheld (TB8) |
| Factory `00809B00` | `set_resource_group`, unkey every table entry then set group |
| Factory `00809B90` | `set_binding_image_data`; source frame progression, binding levels, null data, masked shifts; invalid native memory/work domains explicit |
| Factory `00809CD0` | `reuse_or_create` / `take_from_pool`, oldest matching five-field selection and wrapping transfers |
| Factory `00809E10` | `get_binding_info`, unknown output untouched |
| Factory `00809EB0` | `destroy`, sentinel-conditioned key removal, bulk driver deletion, token invalidation |
| Factory `00809FF0` | `deleting_destructor`, normalized shutdown and bit-0 free decision |
| Factory `0080A010` | `flush_all`, all records including live holders invalidated; exact native table deletion order withheld |
| Factory `0080A0A0` | `adjust_free_pool_size`, unsigned comparison, oldest eviction, saturating trim subtraction only |
| Factory `0080A110` | `reuse_or_create` creation branch, zero frames, wrapping size/accounting and exact failure deletion count; native failed-array leak withheld |
| Factory `0080A320` | `get_binding_from_image`, owner reuse/create/upload/acquire sequence |
| Factory `0080A380` | `get_binding_by_key`, cached/keyed creation and resource ownership; dangling token explicitly unsupported |
| Factory `0080A4D0` | `free_binding`, front insertion, wrapping transfers and trimming; invalid duplicate frees remain a portable guard |
| Factory `0080A580` | `set_free_pool_max` |
| Factory `0080A590` | `flush_free_pool`, temporary zero limit and restoration |
| Factory `0080A5B0` | `BindingFactory::new`, published defaults; native bucket allocation/topology withheld |
| Free `00800BF0`, `00809120` | `MATERIAL_CLASS`, `FACTORY_CLASS` identifiers |
| Free `00800E30`, `0080A680` | `MaterialManager::new`, `BindingFactory::new`; native byte-allocation/free helpers withheld |
| Free `0080A700` | `next_power_of_two`, signed domain and explicit native nontermination boundary |

Shared entries outside that subtotal remain real contracts:

- Factory shared depth/group/pool-limit slots: `set_display_depth`, `resource_group` (`00809240`) and `free_pool_max`; no TB2 wiring.
- Manager shared AddRef/Release (`00800C30`, `00800C40`) and their adjustors: foreign-base delegation; shared-false slot 4 (`005BE2E0`) and unnamed slots 6/7 (`0062C980`) implemented without invented successful behavior/names.
- Material release `008045B0`: `MaterialArena::release`; material-reference assignment `008048F0`: `assign_reference`; material acquisition: `Material::acquire`.
- Value assignment `00802240`: `assign_value`; stage assignment `00802070`: `TextureSlot::assign`; forward range cleanup `008021F0` and temporary reverse-stage cleanup `00805676`: `release_value_range` / `release_stages`.
- Inherited system-service behavior stays foreign behind `SystemService`; no inherited/shared body is counted twice. Bits-per-texel and vtable data are not bodies. TB1/TB2, distinct-record restore ordering and exact native invalid-memory/allocation paths prevent whole-owner credit.

### Final validation and evidence

- **PASS:** canonical `qbuild all`, 2026-10-22 **21:19:44–22:32:51 UTC**, exit 0. Exactly one new full operation ran after the successful touched-file format check; no new quick build, wrapper/cache/config change, skipped test or timeout change was made.
- Cargo: **5098 passed / 0 failed / 0 ignored**, summed over **21** `test result:` lines in fresh `qbuild-test.log`. All **48** texture/material tests passed; the independent S3D material-reader integration also passed. These establish the normalized supported domain described above, not native glue or whole-owner parity.
- Clippy: **PASS**, `-D warnings`; completed dev profile, **zero diagnostics**, 6m 17s. Fresh CMake configuration and generation completed (3.4s / 1.0s); native build completed **214/214** actions, including `FreeCity4.dll` and `fc4_plugin_load_test.exe`, with **11 MSVC warnings / 0 errors**. Warnings are in unchanged plugin/SDK files (`FC4DatesProbe.inc`, `cISC4CitySituationAdvice.h`, `FC4DemolitionHost.cpp`, `cFC4NetworkTool.cpp`, `FC4MySimsShadow.cpp`); this lane did not change them. Release Rust build completed in 11m 36s.
- Full CTest: **68/68 passed**, 0 failed, 1515.17 seconds. The built-in `cargo_test` executed the workspace in CTest's separate build directory as part of this same canonical operation; no additional lane-submitted build was used.
- `qbuild fmt`: **PASS**, exit 0, on the six touched absolute Rust paths (five owner files plus `engine/formats/src/s3d/tests.rs`). The wrapper reported a path-canonicalization warning, not a formatting diff. A preliminary relative-path invocation stopped before launching all; its setup failure and unchanged source were retained in `stable-gate-1/`.
- Hidden supervisor **34972** and qbuild **3376** have explicit start/finish records. Gate HEAD **5a248e549db236251d93754a3911ac3bb24ef9f5**, tree **6314c707ba7ceae49797302e52505bee68f38174**, and engine tree **d734a0c0d45f021fde3d030315cdbec94fcd0dc9** remained identical through completion. Per-file hashes and clean engine status also matched; `source_unchanged` is **true**. The final report commit changes documentation only.
- Durable evidence: `build-runs/contractfix4-r2-evidence/stable-gate-final/{completion.json,source-provenance.json,measured-summary.json,final-logs/}` plus separate fmt/all stdout, stderr, PID and exit records. Only fresh logs from this operation supply the final measurements; preserved quick/configure/interrupted logs are excluded.
- Initial native process audit at **21:16:40 UTC** found prior PIDs **34944/15340/2848 absent**, with no attributable build supervisor or descendants. Session 41229 was not proof of liveness. The old `qbuild-test.log` was **521 bytes**, mtime **21:06:39.852239 UTC**, without completed results. Commandline/cwd/creation-time evidence is in `initial-process-audit.json`; intermediate PID identities and descendants are retained in the other audit JSON files. Python psutil was unavailable and CIM denied; read-only native queries succeeded.
- Historical failures remain distinct: baseline all was interrupted after **639 passed / 0 failed** completed results with the C API clock test stalled; no baseline clippy/native pass is claimed. Old quick was **3044 passed / 3 failed**, from stale bulk-delete event assertions. First stable all on `f471bdc8` stopped at **E0046**, missing `destroy_base` in the independent reader mock. **399d555d** corrected the mock and all three assertions without production reader changes or skips. Original reports/commit provenance and available interrupted/mixed logs remain preserved under `stable-gate-1/interrupted-before-gate/` and `stable-gate-final/interrupted-before-gate/`; none are promoted to final evidence.
- The report checklist independently reconciles **all 58 published bodies**, with **0 missing / 0 extra**: 53 class bodies/4749 bytes and 5 free functions/134 bytes. Evidence is `build-runs/contractfix4-r2-evidence/inventory-reconciliation.json`; shared entries are separately enumerated above and do not inflate that subtotal.
- Distinguishing vectors cover signed rounding/nontermination, masked/wrapping LOD and products, zero levels/frames, 65536-square size zero, binding ref underflow/overflow, each wrapping pool counter, stale-zero reuse/eviction/alias restore, first/third-frame failure and prior zero handles, ignored upload errors/null pointers, material counts 0/1/2/65535, equal/different binding assignment, reverse-stage releases and forward range order.
- Game runs: **none**, per this final correction brief; no game/shadow/ref-warning verdict is claimed. C ABI exports and ABI version are unchanged. No maps/default/native-binding or whole-owner credit is requested.

### Questions and next steps

**TB8** remains appended to `docs/specs/questions/texture-bindings.md`; **TB1/TB2** also remain open. No new question or published answer was added in this resume. Distinct-record restore still reports `Unsupported::RestoreTableOrder` before load/upload; no hash/growth/chain traversal is invented. Coordinator should regenerate the question index (engine lane cannot run tools scripts), review the bounded queued source, and retain the original parent evidence. This final logical round is complete; no continuation round or renamed lane is proposed.
