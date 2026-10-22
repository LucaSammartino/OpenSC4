| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-dbpf-native4: independent public packed DBPF transport | `verify-dbpf-native4` / detached engine lane | in progress: round2/3 from b3e8d72d; prior cargo4962/clippy0/CTest68/68; native turbo+normal clean2032/ref0; EX54/EX55 open | 1. Split EX55 transport diagnostics. 2. Run own/current40 saved-city reload and ref0 controls with frozen artifacts. 3. Full gate and bounded report. |

## Handoff

- Goal: round2/3; separate public record transport from original packed owner, preserve EX54/EX55.
- Starting tip b3e8d72d; detached HEAD, no refs/branches/config changes allowed.
- Round1 production implementation d25b2ebd; report b3e8d72d; original attempts retained below and in build-runs.
- Own profile: replacement clock24 only, dbpf_native=1; original packed-segment remains independently owned.
- Current40: literal coordinator list from round2 brief; no terrain/effects substitutions.
- Baseline qbuild all PASS cargo4962/clippy0/CTest68/68 (tree changed: handoff documentation); focused quick PASS cargo531/build4/4.
- Split EX55 list status/key-vector/count diagnostics implemented; combined mismatch counting unchanged; scope log disclaims file ownership.
- Diagnostics committed 02d08687; stable full qbuild all PASS cargo4962/clippy0/CTest68/68; fmt absolute path PASS.
- Frozen build-runs/dbpf-r2-frozen-02d08687/FreeCity4.dll SHA256 3d1eee0c6a7d55b992484c71a7373def98edc30364431198ce649a0bf21966d4.
- Matching PDB SHA256 bc8ceb0de7490779087d30af0993071a208f9cfdfa6edbba61310160d74e00ad; both readonly, hashes in manifest.json.
- Completed vdbpf4-r2-own-archive-normal-02d08687 clean2032/ref0/save12719ms/shutdown; natural60440/60440 and constructed10/10 exact transports.
- EX55 split: constructed list_false1/key_differences1/count_differences0; natural list_false0/key_differences1/count_differences0; combined2+1 preserved. Missing DIR key in both nonempty mismatches.
- Completed own-save-on clean2035-01-12/ref0/save11500ms/shutdown; saved fixture153/153 transport exact (compressed80), constructed10/10; combined EX55 differences1+2 unchanged; refusal0/failed0.
- Own-save-off-02d08687 exited(code1) without verdict after58s; last day2033-11-15; no native verifier lines, warnings/foreign/SEH0 in harness summary. Preserve failure; no clean/ref0 claim.
- EX55 appended with measured split predicates and own saved153 evidence; EX54 unchanged, no new question IDs or spec assumptions.
- Outstanding own-save-off-normal-02d08687 (helper session96187, same fixture/DLL, changed pacing). Next current40-save-on/off sequentially.
- Supported fixture_city imports existing saved fixtures commands4-own-e50c5bbc.sc4 and commands4-r2-house40.sc4; do not claim new save roundtrip.
- Round1 turbo and normal both natural60440/60440 + constructed10/10; three EX55 combined differences each; refusal0/failed0/fallback0/resync0.
- No game-attempt cap: originalcap3 meant lane rounds, this is round2/3.
- Guard manifest C:/Users/tlori/FreeCity4/build-runs/run4-verify-dbpf-native4-r2-manifest.json.

## Previous round report (preserved)

## Result

The production Rust DBPF parser/index/lookup/raw-read/QFS-read path is independently compared with the original packed-file segment acquired through public SDK factory/QI. This lane is WIP because the required current exacthouse40 profile was not supplied in the brief or permitted coordination script, and no separate Rust packed-segment replacement profile is published: data-formats section8 says plugin mode uses the original framework classes. The Rust side was exercised directly in both completed runs; neither run is represented as a separate native packed-segment replacement.

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

Record-open failures0, refusals0, fallback0 and resync0 in each origin and run. The key-list differences are the empty fixture, QFS+DIR fixture and natural archive, preserved under EX55; the current combined diagnostic does not identify which of return status, key vector or record count differed. No overall exact-key-list or whole-owner promotion. No reload claim.

## Questions and remaining work

- EX54, `docs/specs/questions/data-formats.md`: SDK ReadRecordRaw return and in/out size/error contract. Observed successful return/inout pairs4/4 and0/0 (the latter is a positive zero-record check), natural1647/1647 and1665/1665. These observations do not establish the complete failure contract.
- EX55, same file: all three native key-list/count differences, reproduced at normal speed. Keep original and Rust results intact; split return/key-vector/count diagnostics before any narrower conclusion.
- Coordinator must supply the current exacthouse40 replacement profile; never substitute the obsolete terrain/effects/dispatch list. Also define a supported distinct own profile if one exists; DBPF has no published replacement key in this brief.
- Two original game requests completed; none outstanding. Original cap3 leaves one slot. One guarded round only; no restart, merge, rebase, push or subagents.
- Reproduce settings: `dbpf_native=1`, `dbpf_native_fixtures=<absolute WT>/build-runs/dbpf-native-fixtures`; optional `dbpf_native_file` otherwise locates installed SimCity_1.dat. Save script `build-runs/dbpf-native-save.txt`. Requests/verdicts/logs retained under owned build-runs.
- Commits:166fab48 initial handoff;d25b2ebd implementation;1c53058b SDK return review/questions;f86c8f00 stable gate;0515bcf4 frozen native request;26e03ad2 native result/EX55.
- Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-verify-dbpf-native4-r1-manifest.json`. Queue state WIP; no ready claim.
