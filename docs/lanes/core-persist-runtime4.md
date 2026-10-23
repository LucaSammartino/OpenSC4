| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-persist-runtime4 (portable persistence runtime) | `core-persist-runtime4` / detached HEAD (engine lane) | ready, bounded source: cargo 5098/0, clippy clean, CMake 4/4, CTest 68/68; six owners partial; ABI unchanged; game n/a | Coordinator combined integration gate; retain PR1-PR3 and PR9-PR13; no native/shadow/default promotion |

## Result and source

Algorithms from `7921d6fe`, with scoped owned formatting committed as `f32318dbde2a5c675eab3907e23e76fd9104b72d`. Final all ran on that committed source. Its engine tree is `35d3aa7e613c6f0b87eb21cdf827e701be0a0147`; subsequent checkpoints change only this lane document, and `git diff --quiet f32318db..HEAD -- engine` returned 0 after validation. Quick's earlier tree-change annotation was caused by AUDIT documentation: the pre-format diff from 7921d6fe to R1 HEAD contained only AUDIT.md. R2 formatting changes eight owned .rs files; lib.rs, qfs.rs and unrelated descendants retain their contents/order. No algorithm expansion occurred in R2.

Original milestones: f8f00f3c records independent vectors before implementation; 31438dbe adds records/fields/views; ee08730a adds variants, sealed factories and images; 7921d6fe adds concrete partial-I/O/file/module/QI/row cases. Later lifetime/BMP/variant/edge expectations are not claimed test-first.

## Measured validation and retained failures

- Initial mixed baseline retained intact in `build-runs/runtime-gates/baseline-mixed-r1-retained/`: 20 successful suites / 5080 passed cases / 0 failed cases, followed by real formats doctest E0433. Clippy completed separately; CMake reached 214/214; CTest stopped at 20/68 without final total/exit. This was a failed/incomplete combined baseline, not new-runtime execution proof. Fresh native process snapshots found no attributable old checker before restarting; no disappearance cause is inferred.
- Coherent quick completed exit 0: 12 suites / 3225 passed / 0 failed / 0 ignored; core and formats tested, capi/conformance compiled with --no-run, CMake 4/4. Evidence: `runtime-gates/quick/`, `quick-console.log`.
- Fmt failures retained: unscoped 1848 diff lines (baseline module order/child drift plus owned formatting); scoped 574 owned diff lines. Counts/first diagnostics survive in `fmt-console.log` and `runtime-format-patches/fmt-scoped-first-failure.log`. Only owned formatting was applied through apply_patch. Rendering-helper recovery notes retain Bash access denial and corrected UTF-8/filename handling; source was untouched before corrected rendering.
- Scoped fmt then passed exit 0 for all 12 absolute touched-file paths, using skip_children=true,reorder_modules=false to preserve additive shared lists and avoid unrelated traversal. Wrapper's “14 files” counts the two configuration argument tokens.
- ONE completed final all on f32318db: cargo test --workspace --target i686-pc-windows-msvc, **22 suites / 5098 passed / 0 failed / 0 ignored**, including formats doctests; clippy --workspace --all-targets -- -D warnings exit 0 / 0 diagnostics; CMake **4/4**; full CTest **68/68**, 0 failed, Total Test time **497.35 s**. qbuild and fc4verify both PASS; helper exit 0. CTest's fresh final total replaces no historical evidence. Archives: `build-runs/runtime-gates/all/`, `all-console.log`, `status.json`.
- Final fc4verify explicitly reported **[the tree changed during the run]**. The change was confined to this lane report; the checked engine tree remains identical. This documentation-only flag is retained, not interpreted as a source failure or silently dropped.
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

Existing lane questions **PR9-PR13** remain in `docs/specs/questions/persistence.md`: constructors/failure outputs/reentrancy; malformed strings/partial bytes; dimensions/BMP/colour conversion; malformed sizes/QFS framing/failure cleanup; shared stream errors/file-close result precedence. Retain **PR1-PR3**. No new R2 or R3 questions.

Game runs: n/a, explicitly excluded for this portable-core task. No game/native probe, live-user-file action, network, native/shadow/replaced/default promotion or ABI change. Ready applies only to the tested bounded source. Coordinator must run the next combined integration gate; resolve questions and implement native adapters/hosts before broader promotion.

## Final review (original round 3/3, 2026-10-23)

Started clean at `f2d7da6a4805ebe1b1272c994eb3ab0d2de37f16`. Read the copied coordinator final review and R2 result. Recomputed all eight retained SHA-256 hashes (test, clippy, native build, CTest, execution summary, status, all console and scoped fmt console): all match that review. Independently recounted the quick/final totals and all 18 named runtime successes once per run from the raw logs; final CTest total is 497.35 s. Engine trees at the tested source and current HEAD both equal `35d3aa7e613c6f0b87eb21cdf827e701be0a0147`; engine diff is empty and working source is clean. Only this report changes in R3. No quick/fmt/all, game request, source/test change, cache RPC or process action was performed for final reporting.

R2 queue creation was denied at `2026-10-23T00:15:34.122706Z`: **PreToolUse LANE GUARD - CONTEXT BUDGET context151k over150k**. R2 obeyed, committed its handoff at f2d7da6a and returned continue. No queue existed at R3 inspection; this final original round creates the normal queue only after its own report commit. The earlier denial remains evidence, not a prior successful enqueue or a failed test. No fourth round, rename, ref update, merge/rebase/reset or push.

Read-only process metadata refreshed the coordinator's dated observation. At 00:22:37 UTC, inspection Python PID 36248 (created 00:22:37.685713, parent PowerShell 26540 created 00:22:37.457155, parent runner 28308 created 00:22:37.372730) had this worktree as cwd; argv was `"C:\Program Files\Python312\python.exe" build-runs/final-review-processes-r3.py`, launched by that PowerShell's inspection command. Cache daemon 24128 (parent 15964, created 2026-10-22 23:49:34.380758 UTC) had cwd `C:\Users\tlori\.cargo\bin\` and argv `"C:\Users\tlori\.cargo\bin\sccache.EXE"`; its only observed child was conhost 24008 (created 23:49:34.385736, cwd `C:\WINDOWS\`, argv `\??\C:\WINDOWS\system32\conhost.exe 0x4`). Neither was disturbed.

At 00:25:01 UTC, expected R3 supervisor Python 29720 (parent 12644, created 00:20:06.891932) was identified by argv naming the lane backend and `run4-core-persist-runtime4-r3-manifest.json`; its cwd read was denied. Additional daemon 35784 (parent 14204, created 00:21:14.985130) and service descendants were inspected: compiler/cargo argv targeted `train-integration`, not this lane. Foreign-session cwd reads returned access denied; all queried candidate argv were available. No attributable lane build/test worker was found; no claim of global process absence or daemon idleness is made. These observations are dated metadata, not permanent absence guarantees. Python psutil was unavailable, so metadata used read-only Windows APIs; a PowerShell relative-path display method was unavailable and corrected for hash display. Neither affected source or retained gates.

The queue describes the full persistence and resources change as kind engine, ABI false, no game runs, state ready, with the actual clean committed HEAD. Readiness is bounded by the partial-owner and oracle limits above. Combined integration remains coordinator work.
