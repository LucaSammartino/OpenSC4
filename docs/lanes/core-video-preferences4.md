| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-video-preferences4 (bounded portable panel owner) | Codex / core-video-preferences4 | ready: SAME R2 canonical all PASS; cargo 5526/0, 27/27 panel fixtures, clippy 0, CTest 68/68; source unchanged | Land bounded portable owner only; VP1-VP5 remain unsupported; no native panel/game credit |

## Implementation and scope

Code/tests commit: `1244c283`; base `8e95854da8454d89131af5b7f1735f02c7b0cae2` includes authoritative publication `48a401f2`.
The only engine edits are the new `engine/core/src/video_preferences/` module and one additive module declaration at the end of `lib.rs`.

- Raw P storage preserves bytes, little-endian dwords and untouched padding. The six independent u16 panel baselines, sticky C, first-redraw byte, guard and initialized byte are explicit inputs. The constructor is a portable raw-state boundary, not a native constructor claim (VP3).
- The owner implements all 29 readback operation rows: repeated root acquisition, checked AL normalization, last-checked overwrites, first-checked loop exit, checkbox zero assignments, ordered stores and marker comparisons. Utility calls retain the previous interface; next root precedes previous Release; final interface Release is explicit.
- Qualified message admission owns unsigned raw-ID ranges, source/root/+11c tests and forced type-3 true AL. Unknown admitted actions return typed Unsupported, not invented success. All four published action IDs have their qualified branches; valid-App/non-null-P Auto/redraw stops before unqualified work (VP1/VP4).
- Save precedes Apply using a stable typed P+f0c payload identity. Foreign return channels are ignored where published. C-nonzero confirmation requests raw widgets in order and avoids App/notification/close. EndDialog does not roll back owner stores.
- EndDialog owns guard, root/parent callback order, host clear-before-Release and virtual owner AddRef/tail-Release effects. Its guard-zero AL and tail EAX are distinct. QI/Shutdown/direct shared Release model only published behavior. Direct Release takes an explicit raw count snapshot; initial count and AddRef arithmetic are not guessed. Logical free forbids later owner operations; Rust Drop emits no foreign cleanup.
- Normal DoModalWin owns pause, raw service query, retained root, ignored manager result, conditional focus restore, EndDialog and cleanup order. Required service failure and foreign reentry are Unsupported, without invented recovery.
- Redraw is partial: null-P complete path; first enum group through 0079020d returns the retained interface to its unimplemented continuation; separately qualified depth-tail entry accepts an incoming retained reference and performs final baseline capture/cleanup. No synthetic first-slice final Release.
- Show is partial: qualified guard/selector early returns, prefix stopping before first redraw, first three bindings as a qualified slice, and separately qualified false-AL tail. Middle bindings and full initialization remain unsupported (VP3).
- No C ABI/plugin/config/generated-entry changes, native layout, foreign algorithm implementation, catalogue/device/default data, mode enumeration or game request.

## Portable evidence

`fixtures.rs` independently transcribes VP-T1 through VP-T24 before production comparisons. T1-T3 compare only their four-row scoped projection; they are not full-readback expectations. T4-T7 cover full asymmetric mapping, failed lookups, independent words, sticky C and ordered resolution overwrites. T8-T24 cover the published action/message/lifetime/partial-entry domains. Additional tests assert high u16 baselines, store-before-marker order, utility AL=2/255, first-checked no-later-lookup behavior, invalid/null/alias/reentry qualification, raw-channel handling, reference chronology, modal focus paths and qualified show/depth slices.

Actual portable results: **27 video-preferences tests PASS** in the focused core run. The native public vectors remain **PROPOSED / UNEXECUTED**. No complete 14,965-byte owner, default-then-close, native execution, or game/shadow coverage is claimed.

## Stable final qualification (SAME original R2 of 3)

The existing engine implementation required **no source changes or diagnostic fixes**. Exactly one canonical `qbuild.py --wt <WT> all` ran on clean committed HEAD `923944170d8b4f96812473cf04c04c3d5062a136`, from **2026-10-23T06:04:41.392850Z** through **06:09:24.753474Z**. The same yielded exec session **31315**, qbuild PID **34960**, was collected to actual terminal **exit 0**; PowerShell explicitly returned `$LASTEXITCODE`. No duplicate checker, process termination, cache RPC/restart, configuration change, baseline, quick rerun, focused duplicate or game request occurred.

- Cargo workspace: **5526 passed, 0 failed, 0 ignored**, over **all 26** `test result:` rows. Ordered passed counts: **540 + 3 + 1 + 31 + 27 + 40 + 7 + 2 + 1232 + 3429 + 21 + 13 + 19 + 147 + 0 + 2 + 1 + 3 + 1 + 1 + 1 + 5 + 0 + 0 + 0 + 0 = 5526**. Every row has zero failures and zero ignored tests.
- All **27/27** independently named video-preferences tests reran inside the workspace gate and passed; no missing fixture. Exact names, all summary rows, aggregate and log hashes are recorded in `build-runs/video-r2-results.json`.
- Workspace clippy with `-D warnings`: **actual exit 0, diagnostics 0**. Its log also retains the environment's path-canonicalization `warn:` line; this is not a Rust/clippy diagnostic.
- Native CMake via fc4verify: **PASS, 2/2 incremental edges (0.8 s)**. The log records the i686 capi cargo build and generated ABI entries **unchanged (185 interfaces)**. This run did not repeat the historical 217 compile/link edges. No generated file was edited.
- Full native verification: **fc4verify PASS (129.6 s)**, returning success into terminal qbuild exit 0. CTest: **68/68 passed, 0 failed**, complete expected inventory; CTest log elapsed **125.06 s** (wrapper 125.3 s). This qualifies the existing bounded portable build, not native execution of the panel owner.
- Historical owned-file `qbuild fmt` was already exit 0 on the three owned Rust files. No Rust edits occurred in R2, so no formatting or tests were redundantly rerun outside `all`.

## Source binding and durable evidence

R1 entry HEAD was `83e502d6f2fc02c13f34fe09c0ef4d52ae43e170`. The R2 pre-gate and post-gate HEAD were both `923944170d8b4f96812473cf04c04c3d5062a136`, with **empty tracked status** before and after. The checkpoint and final report commits change only this lane report. `1244c283` through qualification has no engine diff.

Engine tree at R1 entry, before gate and after gate: **`75ef2f7165d45907f0f688a76f18d1c3297b1373`**. SHA256 of the sorted path-to-SHA256 map of every tracked engine file's actual bytes: **`0f12bef2b3a9fa01e506a2fdcb14039163cbf2ea4cae99a967b275fb82a5c9a3`**, identical at all three snapshots. Per-file hashes are in `build-runs/video-r2-{inherited,before-all,after-all}.json`; actual wrapper terminal and source-stable assertion are in `build-runs/video-r2-all-terminal.json`. Final report-only tip binding is recorded in `build-runs/video-r2-final.json`.

Canonical log SHA256 values:

| Log under build-runs | SHA256 |
|---|---|
| qbuild-test.log | `fe14290fd6d8c8eac10bf975e8986339feb0308a3528a8816e251fc811df86c3` |
| qbuild-clippy.log | `89754ba308f5a728a6f1bb84b6cb4a9574d20408f3bd04051ed9ae049064641f` |
| fc4verify-configure.log | `b5abc90cabcebb4d3ee75c9c6d83f2ee11f56fb229181b5a837c77360d5a17e5` |
| fc4verify-build.log | `a08606cf427b212282f31603c881cf7ac516a4d0d0304cc8a32744cc09b4f3ef` |
| fc4verify-ctest-list.log | `e24956fde143e36189263a8a310b0c4693add39e0aa198240f1ac4fb41ba33f0` |
| fc4verify-ctest.log | `510ed1443c64f20e07f56db0d5c29b0fea349de66684c226d2359ca64d4a0bbf` |
| video-r2-all-stdout.log | `20f2baba69a8fa6b5742948fffbdce8a135aba34ea6dca1e03a63f3c2ba583f3` |

## Historical R1 evidence (terminal remains UNKNOWN)

Before the canonical wrapper could overwrite logs, **12 exact inherited files** (quick/native logs, own q output including historical Bash/encoding errors, process proof/helper, copied host proof, manifest and checkpoint report) were preserved under unique `build-runs/inherited-r1-r2-qualification/`, with copy-verified hashes in `sha256.json`. In particular, old quick-test SHA256 is `a3d4b814ceb52b673159973732789aa65d8e9786d8281d7358e917677fdd47be`; old native-build SHA256 is `a9ad55c13f01e90b8fdd24fcdded513800d1faeb3aeb2ff52ff5d46660eebbf0`.

Historical focused Rust was **3482/0** (3429 + 21 + 13 + 19 + 0); native progress reached **217/217**, including DLL/load-test links. Old exec **82127** belonged to ended R1 tool context and was unavailable here. Old backend **32756**, qbuild **29272**, fc4verify **36436** and stats child **16164** are **not** credited with successful terminal results.

The inherited process proof records PID 16164 running `sccache --show-stats --stats-format json`, creation Unix time **1792733156.0964632**, own cwd, and **127.0.0.1:55909 -> 127.0.0.1:4620 ESTABLISHED** at **2026-10-23T05:30:45.597037Z**. Copied coordinator host proof at **06:41:22+01:00** records all four PIDs **ABSENT**, listeners **[]**, errors **[]**; its **06:42:26+01:00** named own-cwd/argv snapshot has refs **[]**, errors **[]**. Present absence establishes neither historical exit nor cause. No root kill of PID 16164 is claimed. No isolated global process preflight was repeated.

R2 manifest identity was checked against this exact reserved detached worktree, lane, engine role, round 2 and entry HEAD; copied admission confirms the public-spec dependency and full windows-and-panels authorization. Runtime guards remain binding. The checkpoint commit printed a packed-refs lock permission error but successfully created detached commit `92394417`; no shared refs/config/hooks were changed.

## Questions and next step

No new numbered questions. Existing **VP1-VP5** in `docs/specs/questions/video-preferences.md` remain precise unsupported boundaries; no speculative defaults or fallback algorithms were added.

Ready for the coordinator's normal queue on this bounded portable scope. No new algorithm, owner, API, default, C ABI/plugin/SDK change or foreign App implementation was added. VP1-VP5 still require separately published qualification before expanding their unsupported boundaries. There is no native panel, game/shadow, default-then-close, freezer/reload or whole-14,965-byte-owner claim. No game run or shadow verdict is applicable to this report-only qualification.
