| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-published-conformance4 (public UI/framework conformance inventory) | `Codex` / `verify-published-conformance4` (tooling lane) | ready: public evidence audit of 6 portable owners, 118 operation clauses, 29 vector groups; report checker PASS (538 citations, table parity, 3 draft briefs, own-report-only diff); cargo/clippy/ctest n/a; game n/a; ABI unchanged; owner eligibility/unknowns retained | (1) Coordinator confirm individual metric flags; (2) admit finite verifiers only within original caps and published dependencies; (3) no coverage/native promotion. |

## Report

Six-owner review set: cGZMessageQueue2, cGZMessageQueueManager2, cGZMessageServer2, cGZMessageServer, cRZPath and cRZDirectory. Directory-entry/free helpers are dependencies, not extra owner credit. Milestones: scope record `9b9e2da5`; substantive evidence/constraints `6c0538cb`; final inventory/report in this commit. No WIP implementation is offered.

The published metric is path-based at owner granularity (`docs/COVERAGE.md:65-69`), not a statement that every operation has executable independent conformance proof. `docs/coverage-report.md:107,113` only gives aggregate partial GZ/RZ implementation status. `docs/lanes/core-gz-messages-complete4.md:7,188` explicitly leaves whole-owner completion incomplete for GZ8-GZ10. Its 35-test/full-gate result is historical lane evidence, not a new run here and not native owner conformance. `docs/specs/framework-gz-services.md:380-384` expressly distinguishes published independent vectors from execution results.

The existing path correction lane also records a failed clippy milestone before its later stable pass (`docs/lanes/core-paths-contractfix4.md:11-14`). Its final family cap is 3/3; follow-up proposals must preserve the failure and that cap. `docs/lanes/core-rzdirectory4.md:19-20` describes no coverage-map change and no native behavior proof. Its earlier chapter-6 completeness claim does not close the additional recursive removal contract in `docs/specs/paths.md:248`.

## Public evidence and limits

Reviewed baseline: `640ba764c40a1fd2a959b6c396212fc75bd5736b`, 2026-10-23. This is a tooling/evidence audit, not RE or engine implementation. Only public specifications/questions, SDK interfaces, engine code/tests, coverage policy/report and tracked lane reports were used. No original binary, RE/private notes, answered-question archive, game request, broad build or coverage generator was read/run. Only this lane report is tracked; inventory, complete draft briefs and checker are in this worktree's ignored `build-runs/`.

**Supported:** six specified framework owners have portable code; GZ's four owner weights sum to **11,468 published bytes**. Paths/directories have **zero implemented subsystem credit** (`docs/coverage-report.md:118`). Existing GZ lane explicitly withholds complete-owner credit (`docs/lanes/core-gz-messages-complete4.md:7`). **Incomplete:** additional executable discriminators and published contracts are needed; recursive directory removal is absent. **Unknown:** the public GZ/RZ aggregate `part` rows (`docs/coverage-report.md:107,113`) do not identify individual metric flags. Thus six portable owners are reviewed, but this audit cannot honestly certify six individually zero-implemented metric owners. The coordinator must resolve that eligibility detail from its authorized attribution records; no inferred promotion follows here. Directory's no-map-change report is not proof of its present metric flag.

Policy separates specified, implemented, verified and replaced (`docs/COVERAGE.md:65-69`). A path mapping is not executable conformance, and portable tests do not establish a native shadow or replacement. No byte-weight is allocated to an operation: GZ weights are historical owner totals at spec baseline `ab23a7fa` (`docs/specs/framework-gz-services.md:23-24`). The **2,377-byte** current paths/directories subsystem total and **6,762-byte** historical specified delta (`docs/lanes/re-paths.md:5`) are different aggregates; neither is a cRZPath/cRZDirectory weight or additive to the GZ total.

## Operation inventory

The authoritative machine-readable companion is `build-runs/public-conformance-inventory.json`: named owners, per-operation spec/source/test references, independent vectors, remaining obligations, historical provenance and constraints. Every row below has its own record there. **T** = implemented-and-tested for the stated clause, backed by an executable assertion plus a historical suite report; **I** = implemented-unmeasured for the distinguishing clause; **M** = missing published operation; **F** = foreign/unsupported portable boundary; **U** = unpublished/conflicting contract. T never means every branch, present-HEAD execution, native exactness or whole-owner completion.

Citation keys below resolve to these exact paths; suffixes are **line numbers**, not operation counts:

- G: `docs/specs/framework-gz-services.md`; P: `docs/specs/paths.md`; R: `docs/specs/framework-rz-files.md`.
- Q/QM/S2/S1/REG/GT: `engine/core/src/gz_messages/queue.rs`, `manager.rs`, `server2.rs`, `server1.rs`, `registry.rs`, `tests.rs` respectively.
- PP/PH/PD/PT/PC: `engine/core/src/paths/path.rs`, `helpers.rs`, `directory.rs`, `tests.rs`, `contract_tests.rs` respectively.
- D/DT: `engine/core/src/rz_files/directory.rs`, `directory_tests.rs`.
- CV/CB/CN: `engine/conformance/vectors/paths_owner_buffer_lifetime.json`, `paths_split_boundaries.json`, `paths_full_and_normalize.json`.

| ID | Published operation / discriminating clause | State | Spec; source; executable evidence |
|---|---|---|---|
| Q01 | Queue2 constructor, empty/zero | T | G:35; Q:70; GT:211 |
| Q02 | AddRef | T | G:39; Q:95; GT:268 |
| Q03 | Release at 2/1/0 | T | G:39; Q:103; GT:270,276 |
| Q04 | PushBack, first pulse | T | G:48; Q:124; GT:213 |
| Q05 | PushFront, no extra pulse | T | G:49; Q:135; GT:215 |
| Q06 | Remove first type | T | G:50; Q:199; GT:244 |
| Q07 | Remove first equal | T | G:51,391; Q:204; GT:609 |
| Q08 | Remove last type | T | G:52; Q:209; GT:246 |
| Q09 | Remove last equal | T | G:53,391; Q:214; GT:609 |
| Q10 | Remove all type | T | G:54; Q:219; GT:256 |
| Q11 | Remove all equal | T | G:55,391; Q:224; GT:609 |
| Q12 | RemoveAll count/holds | T | G:56; Q:229; GT:227 |
| Q13 | Front type, empty zero | T | G:57; Q:238; GT:218,229 |
| Q14 | Back type empty zero | I | G:57; Q:243; GT:218 tests nonempty only |
| Q15 | Contains type found/absent | T | G:58; Q:248; GT:219 |
| Q16 | Peek front nonempty hold/count | I | G:59; Q:260; GT:230 tests empty only |
| Q17 | Peek back caller hold, queue retained | T | G:59; Q:265; GT:220-221 |
| Q18 | Peek equal first queued receiver | T | G:60,391; Q:271; GT:609 |
| Q19 | Get front caller hold | T | G:61; Q:277; GT:224-225 |
| Q20 | Get back | T | G:61; Q:282; GT:289,990 via blocking path |
| Q21 | Blocking get front/retry | T | G:62,390; Q:302; GT:281,581 |
| Q22 | Blocking get back failed wait/timeout | T | G:62; Q:312; GT:990 |
| Q23 | Count | T | G:63; Q:322; GT:217,258 |
| Q24 | Destroy close-before-message-Release | T | G:64,97; Q:113; GT:604 |
| Q25 | Get equal caller hold | T | G:65,391; Q:287; GT:609 |
| Q26 | Actual event/wait/lock resources | F | G:72,78,97; Q:302; GT:564,581 are scripted |
| Q27 | Queue QI and remaining body manifest | U | G:43,409; Q:70; GZ9/GZ10 |
| M01 | Manager constructor | T | G:103; QM:29; GT:297 |
| M02 | Full QI counted out/base fallback | F | G:108; QM:35 predicate only; GT:300 |
| M03 | GetQueue caller hold/absent | T | G:109; QM:53; GT:313,315 |
| M04 | AddQueue absent/duplicate | T | G:110; QM:61; GT:309-310 |
| M05 | CreateQueue absent/present | T | G:111; QM:73; GT:297 |
| M06 | Remove pointer with same queue under several IDs | I | G:112; QM:86; GT:316-317 only unique queue |
| M07 | Remove by ID found/absent | T | G:113; QM:98; GT:320,326 |
| M08 | RemoveAll state/holds/true | T | G:114; QM:108; GT:321,323; visitation U |
| M09 | Service refs/destructor remainder | U | G:409; QM:24; GZ8-GZ10 |
| S201 | Server2 constructor/defaults | T | G:122; S2:74; GT:350,549 |
| S202 | Full QI counted pointer/base | F | G:134; S2:97; GT:457 predicate only |
| S203 | Shared logical AddRef | T | G:156,388; S2:130; GT:549 |
| S204 | Service Release | U | G:135,155; S2:136 is RemoveRef, a different contract |
| S205 | Count getter/zero RemoveRef | T | G:145,388; S2:125,136; GT:552,553,560 |
| S206 | Shared AddRef dword wrap | I | G:388; S2:130; GT:549 has no max-count setup |
| S207 | Send exact+wildcard retained snapshot | T | G:211,392; S2:147; GT:361,653 |
| S208 | Post front/back | T | G:228; S2:176; GT:350 |
| S209 | AddNotification duplicate/deferred | T | G:166; S2:186; GT:361,716 |
| S210 | RemoveNotification unlink-before-Release | T | G:171,393; S2:191; GT:686 |
| S211 | Directed append/copy/temporary holds | T | G:246; S2:198; GT:967 |
| S212 | Directed cancellation prefix/suffix defect | T | G:281,397,398; S2:212; GT:786,1039 |
| S213 | Tick ordinary drain and directed quota/nesting | T | G:228,399,400; S2:257,274; GT:350,814 |
| S214 | Posted count | T | G:143; S2:313; GT:557 |
| S215 | Set drain-all | T | G:144; S2:318; GT:350 |
| S216 | CreateMessage class/IID result | T | G:309; S2:324; GT:458-460; host factory only |
| S217 | Invalid class leaves native out sentinel untouched | F | G:311; S2:324; GT:460 None is insufficient |
| S218 | Destroy logical hold totals | T | G:316; S2:333; GT:453; reentry/type order U |
| S219 | Flush still-linked front/global guard | T | G:174,200; REG:87; GT:686,1016 |
| S220 | Apply pending before depth increment | T | G:177,205; REG:87; GT:716,906; clear visibility U |
| S221 | Native locks/service/platform wrappers | F | G:123,192; S2:74; no portable native proof |
| S222 | Owner manifest/cleanup remainder | U | G:409; S2:333; no 38/38 claim |
| S101 | Server1 constructor/defaults | T | G:319,359; S1:68; GT:522 |
| S102 | Full QI +0x1C/base | F | G:319; S1:80 predicate only |
| S103 | Service AddRef/Release | U | G:319; S1:42; GZ9/GZ10 |
| S104 | Send exact type, no wildcard, type zero ignored | T | G:362; S1:111; GT:507 |
| S105 | Snapshot per-callback Release, 385 limit | T | G:365; S1:111; GT:507,522; beyond limit unsupported |
| S106 | Post copied 16-byte value front/back | T | G:359; S1:136; GT:522 |
| S107 | Pending-add transfer and duplicate leak | T | G:340,401; S1:145; GT:883 |
| S108 | Raw pending remove without target hold | T | G:341; S1:149; GT:1076 |
| S109 | Directed copied value/raw target | T | G:352,395; S1:154; GT:736 |
| S110 | Directed cancel prefix/suffix defect | T | G:397-398; S1:162; GT:786 |
| S111 | Tick bounded/all posted drain | T | G:335; S1:179; GT:522 |
| S112 | Directed shared signed quota/nesting | T | G:399-400; S1:195; GT:1076 |
| S113 | Posted count | T | G:336; S1:209; GT:522 |
| S114 | Set drain-all | T | G:337; S1:214; GT:522 |
| S115 | Flush versus direct apply pending adds | T | G:342,345,401; REG:87; GT:883 |
| S116 | Destructor reentrant registry/pending ownership | U | G:319,409; S1:219 logical clear only; GZ8 |
| S117 | Raw native lifetime/locking | F | G:351,358; S1:21; GT:736 external live holds |
| S118 | Remaining 81-body manifest | U | G:21,409; S1:42; GZ10 |
| P01 | Path constructor | T | P:115; PP:26; PT:288; CV:8 |
| P02 | SetPath failure preserves buffer/resplits | T | P:116; PP:44; PT:319; CV:30 |
| P03 | Destroy five strings | I | P:117; PP:9 storage only; no destruction-order witness |
| P04 | GetDrive/FixDriveString | T | P:118,132; PP:54; PT:240; CV:21; native locale F |
| P05 | GetDirectory/FixDirectoryString | T | P:119,134; PP:59; PT:240; CV:22 |
| P06 | GetFileName stored | T | P:120; PP:64; PT:302 |
| P07 | FileName lazy reload | T | P:121,403; PP:68; PT:305; CV:18 |
| P08 | GetExtension/FixExtensionString | T | P:122,136; PP:75; PT:307 |
| P09 | SetFileName counted equality/split | T | P:123,166; PP:80; PC:37; PT:301 |
| P10 | SetExtension split/unsplit | T | P:124; PP:88; PT:298,306; PC:37 |
| P11 | SetBuffer masks | T | P:125; PP:100; PT:313; CV:10; all 16 masks not located |
| P12 | GetPath independent output | T | P:126,401; PP:123; PT:293,319; CV:24 |
| P13 | GetDriveAndDirectory | T | P:127; PP:129; PT:295; CV:24 |
| P14 | GetFileNameAndExtension | T | P:128; PP:135; PT:297; CV:24 |
| P15 | High-bit native CRT classifier | F | P:158,168; PP:31; PC:9 injected classifier only |
| P16 | Contradictory C:/ example | U | P:393-394; PH:1; PT:71 is not an oracle; PA3 |
| D01 | Directory constructor/filter/dot flags/read | T | P:190; R:440; D:98; DT:148 |
| D02 | Destroy filter/path/list in order | I | P:191; R:448; D:88 storage only |
| D03 | Ensure trailing separator | T | P:192; R:432; D:393; DT:148 |
| D04 | ChangeDirectory read=false/true | T | P:193; R:442; D:147; DT:187 |
| D05 | Set filter empty/star and reread | T | P:194; R:445; D:156; DT:187,252 |
| D06 | GetNthEntry actual borrowed payload/bounds | T | P:195,281; D:138; PT:424; native layout F |
| D07 | Alphabetize supplied strings/stable unsigned bytes | T | P:196,294; D:250; PC:160; DT:374 |
| D08 | Read clear/fail/order/attributes | T | P:198; R:450; D:167; DT:170,187,218 |
| D09 | Dot flags and exact-star synthesis | T | P:204; R:450; D:167; DT:230,252,285; RZ9 U |
| D10 | Append specific names flags 1/2/4/8/16 | T | P:211; R:471; D:201; DT:298,354 |
| D11 | File-name wrapper mask | T | P:209; R:471; D:232; DT:337 |
| D12 | Directory-name wrapper mask | T | P:209; R:471; D:240; DT:337 |
| D13 | Recursive listing/global depth/filter restore | T | P:220; R:492; D:255; DT:407,454,489,509 |
| D14 | Pattern existence first/finish | T | P:230; R:515; D:289; DT:550 |
| D15 | Directory existence appended star | T | P:230; R:516; D:298; DT:550 |
| D16 | Create levels/outer-first rollback | T | P:236; R:520; D:305; DT:578,599,621; RZ11 U |
| D17 | Recursive RemoveDirectoryA | M | P:248,414; D:31; PD:1; no owner traversal/delete-file/error API |
| D18 | SplitDirectoryPath literals/bounds | T | P:260; R:537; D:341; DT:663,688; PC:68; edge choices retained |
| D19 | Native encoding/find/layout adapter | F | P:333; R:427; D:31; PD:38; PT:507/PC:192 fake provider only |
| D20 | Entry extra methods beyond published three | U | P:268,271; PD:14; PC:128 covers only default/copy/name-drop |
| H01 | Dependency IsFullPath | T | P:31,390; PH:1; PT:46; CN:1 |
| H02 | Dependency Fullpath explicit/current base | T | P:37,60; PH:1; PT:71,103,132; conflict P16 |
| H03 | Dependency ResolvePath counted-copy/failure | T | P:70,155; PH:1; PT:5,152 |
| H04 | Dependency Splitpath | T | P:76,398; PH:1; PT:169 |
| H05 | Dependency SplitDirectory signed byte bounds | T | P:89,320; PH:1; PC:68; CB:1; PA9 U |
| H06 | Dependency failed-next retry/cache/finish | T | P:346; PD:47; PC:192; native provider F |

## Independent vectors and execution evidence

The JSON companion records **all 14 GZ-V01–V14**, all eight original paths conformance groups (create/remove split), PA4–PA8 and the RZ split literals. Each retains a literal observation and a concrete executable reference. GZ's specification calls these independent instruction-derived expectations, explicitly **not** results (`G:380-384`). The test names alone are not proof; assertion inspection and suite wiring establish executable fixtures, while tracked reports supply only historical aggregate execution.

Important discriminators are `[B,C,A,A] -> [B,C,A]` after cancel (G:398 / GT:786), current-tick replacement delivery `A,C,D,E,F` (G:399 / GT:814,1076), nested countdown `-1` then `-2` (G:400), generation-1 raw copied four-dword payload with zero reference calls (G:395 / GT:736), and direct-apply duplicate pending-add leakage (G:401 / GT:883). Preserve each; blanket remove-all, borrowed server-2 snapshots or repaired leaks are incompatible. Native invalid-iterator cases (last/sole cancellation), stack overflow above 385 listeners and arbitrary iterator-invalidating callbacks have no deterministic expected outcome (`G:403-407`, G:365-368).

New actionable clauses found by this review: Q14 empty-back result; Q16 nonempty-front caller reference/count; M06 same queue under multiple IDs removed only once at first sorted ID; S206 wrap boundary (the suite's `u32::MAX` occurrences are Wait inputs/results, not a seeded shared count). These are **verification gaps**, not proven implementation defects. Existing lane wording about wrap tests does not substitute for an assertion. A feasible wrap test needs an admitted, independently specified count-state seam; do not run 2^32 calls or change production code covertly.

Path literals retain the contradiction for identical `C:/` inputs (P:393-394, PA3). PA6's byte-boundary values include capacity 0 success/count 0, 254 bytes plus separator succeeding, 255 failing, and an aggregate-prefix limit; PC:68 and CB cover them. P:416-417's recursive removal success/missing-directory results have no executable owner fixture because D17 is absent. The host's single-remove callback is used for create rollback, not recursive removal. Directory owner storage is shared by re-export (`PD:2-6`); this is not two completed owners.

`engine/conformance/build.rs:14-39` generates a test for each JSON file; `engine/conformance/src/paths.rs:60-131` dispatches paths operations. The three paths JSON files are executable fixtures. No GZ JSON dispatcher was located in the searched conformance sources; GZ has Rust unit fixtures. A generated test plus a file is still only a **written/wired fixture** until execution evidence is attached. The final path lane reports its aggregate conformance run, but no per-file output was inspected here.

Historical evidence only:

- GZ report `docs/lanes/core-gz-messages-complete4.md:3`: **35 GZ tests**, cargo **4662/0**, clippy **0**, CTest **68/68** at that lane's commits; no native owner or whole-owner claim.
- Directory report `docs/lanes/core-rzdirectory4.md:3`: **25 directory tests**, cargo **4732/0**, clippy **0**, CTest **68/68**. Its chapter-6 completeness wording excludes other published D17 behavior and cannot be generalized.
- Path correction report `docs/lanes/core-paths-contractfix4.md:8-10`: source `a4971f046eff6abc9ec151e8a658a7454b106a7e`, cargo **5087/0** (core **3054**, conformance **1232**), clippy **0**, CTest **68/68**. Preserve the earlier **3053/1** high-byte failure and **1231/1** split failure; the original split log was overwritten and is not surviving proof (`:13`). The clippy `manual_range_patterns` correction remains recorded (`:14`).

The static check counts **34** `#[test]` functions in current `engine/core/src/gz_messages/tests.rs`, versus the historical report's **35**. This discrepancy is unexplained here; neither number is silently corrected, and no new pass count is asserted. A future verifier must record actual selected case names/counts, not inherit 35 from prose.

No native behavior conformance harness for the six owners was located in searched `engine/tests` and conformance sources. `engine/tests/abi_conformance_test.cpp:127` includes the message SDK; declarations (`engine/third_party/gzcom-dll/gzcom-dll/include/cIGZMessageServer2.h:37-52`) are interface facts, not behavioral vectors. The fake legacy server at `engine/tests/plugin_load_test.cpp:1785-1797` serves dialog/plugin tests and is not the portable server under test. Existing native suite pass counts cannot be called native owner proof. SDK declarations also do not resolve spec return-channel/ownership uncertainties or GZ9.

## Ranked admission candidates

The complete, finite, reviewed drafts are `build-runs/public-conformance-draft-briefs.md`. They are **proposals for coordinator admission**, not dispatched lanes, new implementation rounds or commitments to coverage credit. Rank is descending independently published owner bytes; feasibility is separate.

| Rank | Coherent verifier task | Published bytes | Feasibility / dependency | Smallest meaningful future acceptance command |
|---|---|---:|---|---|
| 1 | Complete published Server1 portable-contract verifier | 5,796 (G:21) | Medium for known surface; full-owner closure blocked by GZ8/GZ9/GZ10 | `cargo test --manifest-path engine/Cargo.toml -p fc4-core --test public_gz_server1 -- --test-threads=1` |
| 2 | Queue2 + QueueManager2 verifier, complete published surfaces and alias/peek gaps | 2,892 (G:19-20) | High for logical contracts; QI/base/destructor manifest remain blocked | `cargo test --manifest-path engine/Cargo.toml -p fc4-core --test public_gz_queue_manager -- --test-threads=1` |
| 3 | Complete published Server2 portable-contract verifier | 2,780 (G:18) | Medium; independent reference trace feasible; wrap seam and GZ8-GZ10 unresolved | `cargo test --manifest-path engine/Cargo.toml -p fc4-core --test public_gz_server2 -- --test-threads=1` |

These test targets do not exist yet; none was run here. Drafts give exact proposed case counts, expected-value sources, allowed test-only paths, blockers, rejection criteria and full operation lists. Their values must be transcribed/derived from published contracts **before** reading implementation outputs. No original-game access, native calls, snapshots of Rust outputs or weakened assertions are authorized. If prerequisites are absent, publish an incomplete verdict and stop; do not build another partial owner fragment or claim complete-owner acceptance.

Path/directory completion is **not an admissible fourth path-family implementation round**. The family is exhausted at **3/3** (`docs/lanes/core-paths-contractfix4.md:7,34`); GZ completion remains **2/3 with one remaining** (`docs/lanes/core-gz-messages-complete4.md:3`). An independent verifier is a different deliverable, not an allowance reset. No new allowance is inferred for Directory. Drafts preserve those limits, original region exclusions, clean-room roles and the active MapView/TextEdit/city-input/prop exclusions. No work or bytes are assigned to excluded region owners.

For any later separately authorized directory completion, D17's minimal independent design is: scripted primitive remove success; nonrecursive failure; recursive errors 2/3/19 short-circuit; missing directory short-circuit; recurse child directories in supplied order, stop on first failure; delete files only after successful child removals; retry parent last. Log primitive calls with externally supplied results and compare to P:248-255, plus literal tree-success/missing-false P:416-417. The present host lacks error reporting and file deletion; publish that host contract first. No source inferred expectations or whole-owner acceptance command exists for the absent operation today.

## Questions and next steps

No new questions filed. Existing blockers were checked in the current published open-question files: **GZ8/GZ9/GZ10** at `docs/specs/questions/framework-gz-services.md:7,10,13`; **PA3/PA9/PA10** at `docs/specs/questions/paths.md:13,16,19`; **RZ9/RZ10/RZ11** at `docs/specs/questions/framework-rz-files.md:31,37,43`. PA4–PA8 are published in paths.md; the stale older lane report calling them open does not override the current contracts. Do not read archived answers to fabricate missing expectations.

Coordinator: (1) resolve individual metric eligibility without changing the audit's unknown flags; (2) review the three briefs against remaining family allowances and publish GZ8-GZ10 dependencies before complete-owner admission; (3) keep path completion parked at its original cap; (4) preserve actual failed/unknown evidence and withhold all coverage/native/default promotion. No game runs or merge dependencies are outstanding for this documentation deliverable.

## Checks actually run

`python build-runs/check_public_inventory.py` printed:

```text
PASS inventory: owners=6 operations=118 states={'F': 9, 'I': 6, 'M': 1, 'T': 93, 'U': 9} vectors=29
PASS citations: 538 references resolve; report IDs/states match; GZ attributed bytes=11468
PASS briefs: 3 finite future targets absent; GZ2/3 and paths3/3 preserved; existing GZ executable test functions=34
PASS scope: only own tracked lane report differs; 3 deliverables ignored; git diff --check clean
NOT RUN: cargo, clippy, CTest, game, original binary, native owner conformance or proposed future acceptance commands
```

This checks JSON shape, unique IDs, line-reference existence, operation/state parity, all 14 GZ vector IDs, byte arithmetic, draft count/commands/absent targets, ignored artifacts and own-report-only tracked scope. It does not mechanically prove semantic correctness or operation execution. Counts are review units, not function/byte coverage percentages. The first checker run failed because it rejected Git's LF-to-CRLF advisory; the corrected checker uses command-local `-c core.autocrlf=false` for `git diff --check` without editing Git configuration. q.py was also tried once and printed `Bash/Service/CreateInstance/E_ACCESSDENIED`; subsequent reads used native PowerShell. No failed check or build was relabeled as a pass.
