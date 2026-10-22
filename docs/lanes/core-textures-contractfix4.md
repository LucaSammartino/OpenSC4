| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-textures-contractfix4: texture/material contract corrections | Codex / `core-textures-contractfix4` (engine lane) | in progress; final correction round 2/2, logical 3/3; source 399d555d preserved; prior gate absent | Preserve interrupted evidence, then one durable serial fmt/full gate; report exact results or park. |

## Handoff
- Final same-name resume: correction round 2/2, logical family 3/3. No fourth round or renamed continuation.
- Source remains 399d555d; previous report ca37f85f preserved in history; initial tree clean.
- Native process audit 2026-10-22 21:16:40 UTC: prior PIDs 34944/15340/2848 absent. No qbuild/fc4verify or build descendants attributable to this worktree; only audit shell/Python were present. Python psutil unavailable and CIM denied; native read-only PID/creation/commandline/cwd query succeeded.
- Evidence: build-runs/contractfix4-r2-evidence/initial-process-audit.json. qbuild-test.log still 521 bytes, mtime 21:06:39.852239 UTC, without final results. Session 41229 is not a surviving supervisor.
- Next: preserve interrupted/mixed logs; hidden durable supervisor runs touched-file fmt then exactly one canonical qbuild all with RUST_TEST_THREADS=1. Source stays fixed; completion/provenance/stdout/stderr remain in own ignored evidence directory.
- Goal: correct existing textures_materials owner to published TB4–TB7 contracts; no native/default promotion.
- Done: reserved detached worktree reused; clean initial status; TB4–TB7 focused contracts read.
- Done: e9ee879f signed power, masked LOD, wrapping layout and binding references (before call 30).
- Done: factory wrapping counters, instance-zero cache/reuse, bulk failure deletion, source-frame/masked-level/null uploads.
- Done: material 16-bit acquisition/release, reference/value/stage assignment, forward range cleanup, manager false slots/base destruction.
- Done: independent distinguishing vectors added; focused quick and fmt invoked, logs pending.
- Done: 2a906396 factory/material correction and independent vectors; formatting passed five touched Rust files.
- Done: order-independent repeated-alias restore now reloads the current record key once per table entry; distinct-record order withheld under TB8.
- Done: f471bdc8 stable source; fmt PASS five files. Baseline all interrupted after 639 passed/0 failed completed results; C API second-thread clock test stalled over 60 seconds. No baseline clippy/native completion.
- Done: quick old snapshot 3044 passed/3 failed (stale bulk-delete assertions); first stable all stopped on missing destroy_base in formats' independent MaterialBase mock. Both corrected; no tests skipped.
- Done: invalid last factory-reference release now explicitly marks Unsupported::InvalidBinding before safe token cleanup. Reader integration verifies material count/header unknown byte.
- Prior worker claimed a live final gate; this resume established its absence, so no passing prior final gate is claimed.
- Facts: build-runs/summarize-contractfix4.py summarizes only gate logs; review record below reconciles every published body/shared entry.
- TB8 filed: native hash/growth/chain rules absent; distinct-record restore explicitly unsupported.
- Baseline wrapper session 62872 interrupted with Ctrl-C; focused quick native build still pending.
- Scope: textures_materials and independent conformance/tests only; no game runs or shared ABI edits.
- Family cap: parent round 1 landed train84; correction round 1 used one; this final correction round is logical 3/3.
- TB1 interface identity and TB2 settings producer remain unknown.

## Review record (validation pending)

The existing production owner remains `engine/core/src/textures_materials/`; no second owner, S3D decoder, native adapter, default routing, ABI export, shared/generated table, glyph code or game file was added. No game requests were issued, as required by this brief. Original parent `core-textures-materials4` remains parked with its train84 evidence. This is final correction round **2/2**, logical family round **3/3**; no fourth round or renamed continuation is available.

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

### Validation record

- Baseline `qbuild all`: interrupted after completed result lines totaling **639 passed / 0 failed**; the C API second-thread clock test stalled. No baseline clippy/native result is claimed.
- During-work `qbuild quick`: old snapshot **3044 passed / 3 failed**; the failures were stale exact event lists missing the added bulk-delete call. All three assertions were corrected. Its native dependent build may still be completing; no current-source pass is claimed from this snapshot.
- `qbuild fmt` on six touched absolute Rust paths (the five owner files and independent `engine/formats/src/s3d/tests.rs`): **PASS**, exit 0; no other Rust files formatted.
- First stable-source `qbuild all` on `f471bdc8` stopped at `E0046` in the independent reader test's foreign-base mock, which needed the new destruction primitive. Mock corrected; no production reader changes. The replacement final stable-source all uses `RUST_TEST_THREADS=1` and is pending. Serial tests avoid the observed baseline interaction without changing other subsystems.
- Replacement full gate source is **399d555d**, session **41229**, still compiling at 22:09:21; no current-source pass or ready queue is claimed. Collect this existing gate, use `build-runs/summarize-contractfix4.py` for bounded exact counts, fix any new failure if necessary, then replace Handoff with the final report and enqueue. No merge-queue JSON has been written while validation is pending.
- Distinguishing vectors cover signed rounding/nontermination, masked/wrapping LOD and products, zero levels/frames, 65536-square size zero, binding ref underflow/overflow, each wrapping pool counter, stale-zero reuse/eviction/alias restore, first/third-frame failure and prior zero handles, ignored upload errors/null pointers, material counts 0/1/2/65535, equal/different binding assignment, reverse-stage releases and forward range order.

### Questions and next steps

**TB8** appended to `docs/specs/questions/texture-bindings.md`; TB1/TB2 remain open. Coordinator should regenerate the question index (engine lane cannot run tools scripts), review this bounded portable correction and retain the parent evidence. No native binding/default or whole-owner completion credit is requested. One final gate result and queue entry remain to be recorded.
