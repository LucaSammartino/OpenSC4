| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-dbpf-native4: independent public packed DBPF transport | `verify-dbpf-native4` / detached engine lane | In progress round3/3: reconcile retained gate/artifact and run one budget-enabled current40 OFF-first matched lifecycle pair; round2 WIP and all eight earlier attempts retained; ABI3 additive exports retained | Verify frozen provenance and OS/queue attribution before any request; no builds or engine edits. |

## Handoff
- Goal: final round3/3, bounded original record/payload transport plus matched budget-enabled current40 lifecycle validation.
- Start: clean detached HEAD5c537c6f; round2 report is committed there and remains below.
- Historical: eight attempts retained; current40 ON/OFF clean/ref4, own OFF-turbo exited1, other five clean/ref0; EX54/EX55 unresolved.
- Done: read contribution, lane/queue and operating docs; no engine/source/default changes or compiler/test invocations.
- Next: recompute stable gate totals/hashes and engine tree; verify immutable DLL/PDB, literal corpus and supplied current40 fixture; inspect queue/process attribution.
- CIM process enumeration denied; use read-only OS metadata fallback. Main queue showed regress-train105-shadows.req, owned queue only done; not proof of live state.
- Requests planned only after provenance passes: vdbpf4-r3-budget-on-current40-off-02d08687-a, then vdbpf4-r3-budget-on-current40-on-02d08687-b; neither submitted.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-dbpf-native4-r3-manifest.json.
- Frozen pair: build-runs/dbpf-r2-frozen-02d08687; existing production implementation02d08687, gate304fdfba.
- No fourth round, rebuild, source fitting, game slot access, qwait, subagents or owner/replacement promotion.

## Retained round2 report (at5c537c6f)

## Round2 result

Bounded WIP. Round2 implements separate native list-status/key-vector/record-count diagnostics in `engine/plugin/FC4DbpfNative.cpp` (commit `02d08687`). Production Rust `Package` entry/find/read_raw/read methods remain the model, reached through the existing C API; no verifier clone, native-sourced index, output fitting or spec-contract assumption was introduced. The prior three combined differences remain three per enabled run and are narrowed under EX55, not waived.

The scope line distinguishes SDK record-adapter transport from the independently acquired **original packed-segment owner**. This does not implement or qualify a packed-segment replacement, an entire DBPF owner, or cRZFile ownership. Factory/QI/borrowed As ownership rules remain those recorded below. No additional C exports this round; the lane's three additive exports remain, ABI version unchanged.

Own mode uses `disable=budget`, replacement `enable=clock24` only, and `dbpf_native=1` (or 0 for controls). Current40 uses the same budget setting and exactly this supplied enable list, plus the verifier when enabled:

`landvalue, weather, ordinance, demand, power, plumbing, flammability, pollution, protection, crime, aura, flora, landfill, residential, neighbors, zones, development, tract, history, clock24, commercial, industrial, civic, parks, lotdev, zonedev, traffic, automata, netmgr, netlot, nettool, demolition, occmgr, propmgr, lotcfg, citysit, propdev, simgrids, citydetail, ports`

Logs confirm all40 enabled keys, with only the mandatory default simulator in addition. No terrain/effects/dispatch substitution. Verifier-off requests retain the identical profile, fixture and frozen DLL/PDB.

## Gate and immutable inputs

- Initial baseline `qbuild all`: cargo4962/clippy0/CTest68/68 PASS; wrapper noted the handoff documentation changed during the run. Authoritative stable final `qbuild all`: cargo4962 passed/0 failed, clippy0 diagnostics, CTest68/68 PASS, no tree-change flag. Full CTest134.23s (wrapper135.5s).
- Focused `qbuild quick`: fc4-capi531 passed/0 failed, build4/4 edges PASS. `qbuild fmt` on the absolute capi/src/dbpf.rs path PASS; no Rust formatting changes. No Rust source changes in round2.
- Frozen pair: `build-runs/dbpf-r2-frozen-02d08687/FreeCity4.dll` and adjacent `FreeCity4.pdb`, both readonly. DLL17491456 bytes, SHA256 `3d1eee0c6a7d55b992484c71a7373def98edc30364431198ce649a0bf21966d4`; PDB107999232 bytes, SHA256 `bc8ceb0de7490779087d30af0993071a208f9cfdfa6edbba61310160d74e00ad`.
- The frozen manifest also records supplied saved-city fixtures under main build-runs/fixtures: `commands4-own-e50c5bbc.sc4`,1018906 bytes, SHA256 `ff38a466b52443575dfc03761f51702169ea996020721b9fe124268300092118`; `commands4-r2-house40.sc4`,998674 bytes, SHA256 `4ad4e47b96e66110216ce0a2794c7f0cd6cdb5450295b97d320e7e94469efff8`.
- Hashes checked before/after requests and in final audit. The documented `fixture_city` route stages writable copies; returned verdicts confirm source SHA256s preserved. Rust and the independent native segment separately open/read-lock the original supplied source for comparison. These are reloads of supplied saves followed by successful full saves, **not a lane-produced save-and-reload roundtrip**.

## Round2 game results (all attempts retained)

All labels below use the same frozen DLL/PDB. All clean verdicts have foreign exceptions0, stalls0, RTC0, overruns0. Ref4 is retained even where the harness calls its verdict clean.

| Unique label | Mode / final date | Verdict and references | Full save / shutdown |
|---|---|---|---|
| vdbpf4-r2-own-archive-normal-02d08687 | own on, normal /2032-01-01 | clean/ref0; natural60440/60440 + constructed10/10 transport | ok1/12719ms; completed |
| vdbpf4-r2-own-save-on-02d08687 | own on, turbo, supplied save /2035-01-12 | clean/ref0; saved153/153 + constructed10/10 transport | ok1/11500ms; completed |
| vdbpf4-r2-own-save-off-02d08687 | own off, turbo, same save /last log2033-11-15 | **exited code1 without verdict**, wall58s; zero verifier lines; harness summary warning/foreign/SEH0, no clean/ref0 claim | not reached |
| vdbpf4-r2-own-save-off-normal-02d08687 | own off, normal, same save /2035-01-01 | clean/ref0; zero verifier lines; changed pacing only | ok1/11172ms; completed |
| vdbpf4-r2-current40-save-on-02d08687 | current40 on, turbo, supplied save /2035-01-01 | clean/**ref4**; saved153/153 + constructed10/10 transport | ok1/11125ms; completed |
| vdbpf4-r2-current40-save-off-02d08687 | current40 off, turbo, same save /2035-01-01 | clean/**ref4**; zero verifier lines | ok1/11063ms; completed |

The current40 on/off warnings match: four last-reference releases on SDK class `0xE990BFFC` (`kcSC4DepartmentBudget` in public GZCLSIDDefs.h), two at cFC4PortDeveloper.cpp DropObject:144, one at cFC4Parks.cpp DropObject:126 and one at cFC4Landfill.cpp ReleaseToken:465. They occur in shutdown outside the verifier and are reproduced with it disabled. This establishes a failing baseline control, not a resolved ownership contract or permission to suppress warnings. No other subsystem ownership code changed.

The own off-turbo exit remains unexplained. The changed-pacing normal control passes; the original failed attempt is preserved and was not resent unchanged. No normal-speed EX55 difference vanished: the archive diagnostics reproduce the earlier native turbo/normal differences.

## Exact transport and retained differences

The following are per enabled round2 run, not aggregate independent corpus counts. Every record includes metadata, independently locked on-disk raw bytes, production Rust raw bytes, native raw bytes and actual native-vs-Rust decompressed payload. Refusal0, record-open failure0, fallback0 and resync0 in all enabled origins/runs.

| Input | Open/reject / index | Record transport | Combined EX55 / split predicates |
|---|---|---|---|
| Constructed, each of3 enabled runs |16/16 decisions matched;9 opened,7 rejected;11 raw entries including duplicate |10/10; compressed1, empty1; zero1/one7/multi1 opened files |2 differences of9 checks; list_false1, key_differences1, count_differences0 |
| Natural archive, own normal |1 opened;60440 raw/unique entries |60440/60440; compressed48880, empty0 |1 difference of1; list_false0, key_differences1, count_differences0 |
| Supplied own save |1 opened;153 entries |153/153; compressed80, empty0 |1 difference of1; list_false0, key_differences1, count_differences0 |
| Supplied current40 save (different SHA256) |1 opened;153 entries |153/153; compressed80, empty0 |1 difference of1; list_false0, key_differences1, count_differences0 |

EX55 split: empty fixture returns false with size0/count0 and equal empty vectors. The QFS fixture lists2/expected3/count3; archive60439/60440/count60440; both supplied saves152/153/count153. Each nonempty mismatch is exactly one missing DIR key `E86B1EEF/E86B1EEF/286B1F03`, with zero extras. All record counts equal the full index. The comparison still includes DIR, still treats empty-list false as the original combined difference, and still tests every record directly.

## Questions, queue state and next steps

EX54 remains open for raw-read return and buffer/failure semantics. EX55 in `docs/specs/questions/data-formats.md` now includes separate predicates, normal-speed evidence and both saved-city comparisons. No new question ID or guessed contract; no spec file or generated question index edited. Spec/SDK do not publish enough to resolve those semantics from implementation work.

Six round2 attempts complete:3 clean/ref0,2 clean/ref4,1 exited. Together with the two round1 clean/ref0 attempts below, all eight attempts remain in owned game-results/game-queue/done. **No outstanding requests or running games.** Queue state WIP, never ready; no default/replaced or entire-owner promotion. The local strict control audit in build-runs/dbpf-r2-audit.log fails on the two ref4 controls and the exited control as intended, while the required build/test gate passes.

Next: published answers for EX54/EX55; coordinator ownership follow-up for the DepartmentBudget baseline warnings; diagnosis of the preserved own turbo-off exit. A true lane-produced save roundtrip additionally needs a documented save-export route; do not access live slot/Document saves or invent save_as semantics. Do not repeat an unchanged failing setup or spend another lane round without resolving the relevant prerequisite.

Round2 started at b3e8d72d. Implementation02d08687; stable gate304fdfba; own normal evidence0b8fdb86; own reload934d44d4; failed turbo af9051ff; EX55 diagnostic evidence d68be757; normal-off f6f843f7; current40 on7fc00060; off/baseline5c478fca. Detached HEAD remains intentional, supervisor publishes the reserved branch. Guard manifest `C:/Users/tlori/FreeCity4/build-runs/run4-verify-dbpf-native4-r2-manifest.json`. The limit is three **lane rounds** total; this is round2/3, with no arbitrary game-attempt cap.

## Round1 evidence (historical; profile gaps superseded above)

## Result

The production Rust DBPF parser/index/lookup/raw-read/QFS-read path is independently compared with the original packed-file segment acquired through public SDK factory/QI. Round1 ended WIP because the then-required current exacthouse40 profile and distinct own-mode definition had not been supplied; those profile gaps were supplied for round2 above. Data-formats section8 says plugin mode uses the original framework classes. The Rust side was exercised directly in both completed round1 runs; neither run is represented as a separate native packed-segment replacement.

## Changes and acquisition contract

- `engine/plugin/FC4DbpfNative.cpp/.h`: off by default, `dbpf_native=1`, triggered once per process at city load. No retained record, segment, probe data or serialized state.
- Factory `GetClassObject(0x086D4A89,0x286D4A6A)` owns the packed interface; QI `0x656B8F0C` owns the persistence reference. The As return is borrowed and compared with QI, without Release.
- Only SDK typed methods are called: Init, SetPath, Open(read=true,write=false), key-list/count, record existence/sizes/position/compression flag, ReadRecordRaw, OpenRecord(Read), GetKey/GetSize/GetFieldVoid, CloseRecord, Close and Shutdown. Key-list factory uses the published CLSID/IID; returned key references are copied, never retained/released.
- All published interface method addresses are checked for committed executable storage in the identified supported game image; table storage is also checked. No private constructor, allocator, fixed-address dispatch, invented vtable or verification DBPF implementation.
- Factory/QI holds are released separately; each successful opened record is closed and its caller hold released; created objects use FC4_RELEASE_CREATED. Measured packed/persistence holds: acquired17/released17 each, net0; key-list holds acquired10/released10, net0. No ref warnings in either run. Unsupported-interface paths refuse further dispatch.
- Three additive C exports in `engine/capi/src/dbpf.rs` and `fc4.h` expose actual Package entries/find/read_raw/read. The complete Rust index is independently parsed from the file, never populated from native enumeration. ABI exports changed; FC4_ABI_VERSION unchanged.
- Existing formats city-load hook invokes the transport probe before the separate exemplar shadow's opt-in guard. This task does not compare exemplars, cohorts or variants.
- Literal C API test covers complete index order, first duplicate wins, zero payload, raw bytes, actual QFS decoding, out-of-range metadata and missing-key failure without emitting bytes.
- MSVC groups overloaded Init declarations: SDK Open occupies slot6 and OpenRecord slot21, consistent with the spec. Scoped [abi-ret] source comments distinguish this SDK interface from the separate SC4 streaming interface; no checker maps or ABI tables edited.

## Independent inputs

`build-runs/make-dbpf-fixtures.py` constructs 16 literal DBPF7.0 files without engine writers or native output. Files are readonly, with individual lengths/SHA256 in `build-runs/dbpf-native-fixtures/manifest.json`; manifest SHA256 `3FD0E623041B954870C2043A10B30582B35092AE4831F19ED6F37A8FD5C70D4B`.

Accepted fixtures include zero records, one ordinary record, one zero-size record, plain+QFS+DIR multi-record input, duplicate keys, an unlisted QFS signature, minor99, index-major99 and index-minor99. Seven rejected fixtures cover major2, out-of-bounds index offset, record/hole count limits, below-header record offset, overflowing record extent and bad magic. Native malformed reads are not attempted: only Open's published rejection checks are used.

Natural input is installed `SimCity_1.dat`, opened independently and read only. A FILE_SHARE_READ-only handle prevents writes throughout each comparison; native and Rust each open the identical locked path. Only the header/index words and stored bytes are read for independent provenance. No installed archive, game executable or Documents file was read by shell or modified. No natural archive copy/hash is claimed.

## Gate

- Initial baseline all was started before code changes, but its link overlapped hook addition and failed with unresolved DbpfNativeCityLoaded. Record it as interrupted, not a clean baseline.
- Focused quick: capi531 tests PASS and CMake PASS. fmt check on the touched Rust file PASS (absolute path required by wrapper).
- First full gate: cargo4962 PASS, clippy0 diagnostics, CTest67/68; two ret_channel_check name-collision flags.
- Stable full gate after scoped SDK comments: cargo4962/4962 PASS, clippy0 diagnostics, CTest68/68 PASS; qbuild all PASS. Source commit `1c53058b`; subsequent commits only update lane/question documentation.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- Frozen DLL: `build-runs/dbpf-native-frozen/FreeCity4.dll`, SHA256 `524CD1243584A4613969B49B28870A0F1CF0D3DF242133B431DA002304288A5A`; same immutable DLL and fixtures used in both game runs.

## Measured game results

| Run | Pacing / final date | Verdict | Full save / shutdown |
|---|---|---|---|
| vdbpf4-r1-native-1c53058b-a | turbo /2032-01-04 | clean; foreign exceptions0, stalls0, RTC0, overruns0, ref warnings0 | SaveCity(full) ok1,11000ms; marked after save; PostSystemServiceShutdown reached |
| vdbpf4-r1-normal-1c53058b-b | normal /2032-01-01 | clean; foreign exceptions0, stalls0, RTC0, overruns0, ref warnings0 | SaveCity(full) ok1,10875ms; marked after save; PostSystemServiceShutdown reached |

The following counts are per run; normal reproduces turbo, not additional independent corpus coverage.

| Origin | Open / reject decisions | Unique record metadata/raw/payload | Compressed / empty payloads | Native key-list/count combined check |
|---|---|---|---|---|
| Constructed |16/16 matched;9 opened,7 rejected; zero1/one7/multi1 |10/10 exact in each category;11 raw index entries including duplicate matched Rust |1 compressed;1 empty |7/9 exact;2 differences |
| Natural |1/1 opened; multi1 |60440/60440 exact in each category; independent raw index matched Rust |48880 compressed;0 empty |0/1 exact;1 difference |

Record-open failures0, refusals0, fallback0 and resync0 in each origin and run. The key-list differences are the empty fixture, QFS+DIR fixture and natural archive, preserved under EX55; the round1 combined diagnostic did not identify which of return status, key vector or record count differed. Round2 splits them above. No overall exact-key-list or whole-owner promotion. Round1 made no reload claim.

## Questions and remaining work

- EX54, `docs/specs/questions/data-formats.md`: SDK ReadRecordRaw return and in/out size/error contract. Observed successful return/inout pairs4/4 and0/0 (the latter is a positive zero-record check), natural1647/1647 and1665/1665. These observations do not establish the complete failure contract.
- EX55, same file: all three native key-list/count differences, reproduced at normal speed. Keep original and Rust results intact; split return/key-vector/count diagnostics before any narrower conclusion.
- Round1 profile prerequisite: exacthouse40 list and own mode were missing. Both supplied in round2; do not substitute terrain/effects/dispatch or infer a new DBPF replacement key.
- Two round1 original game requests completed. Correction: originalcap3 meant three lane rounds, not three game attempts. Round2 results and final outstanding state are recorded above. No merge, rebase, push or subagents.
- Reproduce settings: `dbpf_native=1`, `dbpf_native_fixtures=<absolute WT>/build-runs/dbpf-native-fixtures`; optional `dbpf_native_file` otherwise locates installed SimCity_1.dat. Save script `build-runs/dbpf-native-save.txt`. Requests/verdicts/logs retained under owned build-runs.
- Commits:166fab48 initial handoff;d25b2ebd implementation;1c53058b SDK return review/questions;f86c8f00 stable gate;0515bcf4 frozen native request;26e03ad2 native result/EX55.
- Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-verify-dbpf-native4-r1-manifest.json`. Queue state WIP; no ready claim.
