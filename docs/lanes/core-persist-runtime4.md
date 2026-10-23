| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-persist-runtime4 (portable persistence runtime) | `core-persist-runtime4` / detached HEAD (engine lane) | ready: bounded Rust algorithms; six owners partial; cargo 5098/0, clippy clean, CTest 68/68; ABI unchanged | Coordinator combined integration gate; resolve PR1-PR3 and PR9-PR13 before native/default promotion |

## Result and source

Algorithms from `7921d6fe`, with scoped owned formatting committed as `f32318dbde2a5c675eab3907e23e76fd9104b72d`. Final all ran on that committed source. Its engine tree is `35d3aa7e613c6f0b87eb21cdf827e701be0a0147`; subsequent checkpoints change only this lane document, and `git diff --quiet f32318db..HEAD -- engine` returned 0 after validation. Quick's earlier tree-change annotation was caused by AUDIT documentation: the pre-format diff from 7921d6fe to R1 HEAD contained only AUDIT.md. R2 formatting changes eight owned .rs files; lib.rs, qfs.rs and unrelated descendants retain their contents/order. No algorithm expansion occurred in R2.

Original milestones: f8f00f3c records independent vectors before implementation; 31438dbe adds records/fields/views; ee08730a adds variants, sealed factories and images; 7921d6fe adds concrete partial-I/O/file/module/QI/row cases. Later lifetime/BMP/variant/edge expectations are not claimed test-first.

## Measured validation and retained failures

- Initial mixed baseline retained intact in `build-runs/runtime-gates/baseline-mixed-r1-retained/`: 20 successful suites / 5080 passed cases / 0 failed cases, followed by real formats doctest E0433. Clippy completed separately; CMake reached 214/214; CTest stopped at 20/68 without final total/exit. This was a failed/incomplete combined baseline, not new-runtime execution proof. Fresh native process snapshots found no attributable old checker before restarting; no disappearance cause is inferred.
- Coherent quick completed exit 0: 12 suites / 3225 passed / 0 failed / 0 ignored; core and formats tested, capi/conformance compiled with --no-run, CMake 4/4. Evidence: `runtime-gates/quick/`, `quick-console.log`.
- Fmt failures retained: unscoped 1848 diff lines (baseline module order/child drift plus owned formatting); scoped 574 owned diff lines. Counts/first diagnostics survive in `fmt-console.log` and `runtime-format-patches/fmt-scoped-first-failure.log`. Only owned formatting was applied through apply_patch. Rendering-helper recovery notes retain Bash access denial and corrected UTF-8/filename handling; source was untouched before corrected rendering.
- Scoped fmt then passed exit 0 for all 12 absolute touched-file paths, using skip_children=true,reorder_modules=false to preserve additive shared lists and avoid unrelated traversal. Wrapper's “14 files” counts the two configuration argument tokens.
- ONE completed final all on f32318db: cargo test --workspace --target i686-pc-windows-msvc, **22 suites / 5098 passed / 0 failed / 0 ignored**, including formats doctests; clippy --workspace --all-targets -- -D warnings exit 0 / 0 diagnostics; CMake **4/4**; full CTest **68/68**, 0 failed, Total Test time **497.35 s**. qbuild and fc4verify both PASS; helper exit 0. CTest's fresh final total replaces no historical evidence. Archives: `build-runs/runtime-gates/all/`, `all-console.log`, `status.json`.
- No second final all was run for later documentation changes. Generated ABI index remained unchanged (185 interfaces); no plugin/CAPI/ABI/version/default/map/manager/cache changes.

## Test inventory and oracle limits

Quick and final workspace logs each account separately for **12 main core + 5 edge core + 1 formats integration** tests, each executed once successfully per run. Exact names, locations and discriminating expectations are committed in [AUDIT.md](../../engine/core/src/persistence_runtime/AUDIT.md#measured-focused-test-inventory); generated per-name execution counts are in each retained `runtime-execution-summary.json`.

Main tests cover integer widths/signed bits/both orders; float non-swapping and failed-read swapping; strings; record close-once; shared cursor and buffer references; raw-prefix/v0/v1; BMP palette/padding/write quirks; factory defaults; construction/Read/QI/forwarding cleanup; variants; explicit memory bounds; literal typed reads and host-defined partial bytes. Edge tests cover padded-v1 choice versus correct v2 rows/packed save; failed serial QI; delegated-buffer QI cleanup; legacy strings/key short-circuit; in-memory file access/close/module targets. Formats test `literal_qfs_load_exact_size_failure_and_save_header` uses literal RefPack bytes and decoded pixels, size/signature failures/unlock, and saved headers.

Synthetic conversions establish ordering/byte plumbing, not native conversion. Encoder round trips are supplementary. Fixtures are constructed, not shipped-data coverage (PR2). Constructor native-order is explicit (PR1); no shipped native-order claim. Packed agreement is separate from the padded-v1 correct-copy FreeCity4 choice (PR3). No native shadow, original-base-resource verification or combined manager/runtime integration credit.

## Per-owner status

Published new-owner denominator: **131 functions / 13,721 B**. Every owner remains **partial**; no exact implemented-body byte subtotal is available or claimed.

| Owner | Functions / B | Supported bodies and specific limits |
|---|---|---|
| DBRecord | 9 / 207 | Key/flags/QI/logical references, segment AddRef, close-once release even on false, destructor release. Explicit constructor count; native allocation/reclamation unsupported. |
| DBSerialRecord | 49 / 6172 | Scalar/array integer/float, raw-result/swap quirks, strings/keys/marker-free variants, borrowed shared-position forwarding. Native vtables/error state, allocation thresholds and malformed-memory effects unsupported. |
| PersistResource | 6 / 115 | Atomic logical references, QI, locked key/type. Rust ownership represents allocation; native reclamation unsupported. |
| PersistResourceFactory | 14 / 680 | Locked construction, forwarded routing, key-before-Read, serial/output QI and temporary cleanup. Sealed production BufferResource specialization; other derived owners, reentrancy and native failure outputs unsupported. |
| BufferResource | 32 / 5030 | Buffer ref/QI/get/set, raw dispatch, v0/v1/v2, BMP/QFS/header/row algorithms and unlock paths. Foreign conversion primitive; malformed sizes, negative BMP/native allocation effects and foreign derived interfaces unsupported. Padded v1/BMP row choices explicit. |
| BufferResourceFactory | 21 / 1517 | Target/version stack, construction/type/QI/serial dispatch, temporary load/save, access 1/3, close ordering and module target selection. OS acquisition primitives; native hosts/constructors/close-failure precedence unsupported. |

External primitives remain raw byte I/O/seek/position/size/close, buffer allocation/ref/QI/locks/pixels/foreign colour conversion, OS open/module bytes, and existing formats QFS. Resource/factory/serial/image algorithms remain ours; no opaque owner oracle. Authoritative persistence_manager::KeyList re-export is unchanged, **22 functions / 1071 B separately**, with existing residuals; no extra credit. Manager/cache **109 / 8437 B** and existing verifier/capped completion are untouched.

## Questions, game and next steps

Existing lane questions **PR9-PR13** remain in `docs/specs/questions/persistence.md`: constructors/failure outputs/reentrancy; malformed strings/partial bytes; dimensions/BMP/colour conversion; malformed sizes/QFS framing/failure cleanup; shared stream errors/file-close result precedence. Retain **PR1-PR3**. No new R2 questions.

Game runs: n/a, explicitly excluded for this portable-core task. No game/native probe, live-user-file action, network, native/shadow/replaced/default promotion or ABI change. Ready applies only to the tested bounded source. Coordinator must run the next combined integration gate; resolve questions and implement native adapters/hosts before broader promotion.
