| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-language-native4 (native utility verification) | `Codex` / `verify-language-native4` (engine lane) | in progress | Establish published native acquisition, typed ABI, storage and lifetime within 15 calls. |

## Handoff
- Goal: compare production language utility core/CAPI against independently owned original utilities for all IDs 0..0x2d.
- Worktree reused at detached HEAD; initial status clean; no lane commits yet.
- Baseline full build deliberately skipped per task-specific instruction; focused checks follow code, one stable final full gate.
- Safe path established before call 15: SDK cIGZLanguageManager service 0x441e5070 / IID 0x0054b7d5; GetNewLanguageUtility slot 39 owns count 1; SDK utility slots 0..29; setters slots 31/32 from published ABI, void per spec. Existing version-guarded fc4::GameString + B allocator owns native output buffers (docs/abi/allocators.md Strings); no new executable bindings.
- Implemented private persistent production-core CAPI transport and opt-in language_native_verify plugin probe; unchanged owners/defaults/ABI tables. Source written by call 20.
- Coverage inputs: IDs 0..45, all 256 byte indices and 7 high-byte indices, four name methods, empty/nonempty initial output, queries/charset/country, integer/time/date/formatted/case/compare/QI/init/setters/lifetime.
- Foreign LocaleHost uses DLL CRT snprintf and Windows APIs, separate host counts; LU1-LU3/SL1 observations separate gap counts; native results never feed core input.
- fmt passed on new Rust file (absolute path needed); initial q.py rustfmt invocation refused WSL access, corrected with direct rustfmt on only the new file. Focused qbuild quick running in exec session 57505; logs build-runs/fc4verify-quick-test.log etc. No full gate run.
- Next: finish quick build; correct compile issues, commit focused milestone; run immutable DLL own/native/current40 2032 ref0 full-save/shutdown controls with unique labels and no qwait.
- First milestone source commit 0641be7e; safe-path docs b4c8ac8e. Focused quick PASS: capi530/0, build211/211; corrected quick PASS capi530/0 build4/4. fmt PASS.
- Immutable first DLL SHA256 e0bc89a7cffde639cb337380ac0057cbd13a95ab61127b2f54120d2300ba463b at build-runs/language-immutable/FreeCity4.dll.
- vlun4-r1-own-0641-2032-a clean2032/full save/full shutdown but ref_warnings359 (missing created-reference audit registration, now corrected); all46 final native/core Release0. strict116235/116505 (270 LU2 return-only gaps), host2484/4140, gaps2629/2773, refused0, managerUnchanged1, natural0/0. All raw differences retained. LU4 filed for foreign printf differences; LU1 year and LU2 returns remain open, no core changes.
- Outstanding unique request vlun4-r1-native-0641-2032-b, same old immutable DLL, normal speed native control; wait session11000, build-runs/wait-language-verdict.py (not qwait). Already completed own verdict is valid and retained.
- Corrected code registers GetNewLanguageUtility's owned creation hold with FC4_ACQUIRED; all case-return comparisons categorized LU2 gap. First fullgate attempt interrupted before ctest due these findings (cargo/clippy logs retained), not a stable fullgate.
- Next: await native verdict; run corrected immutable own/native/40-key controls, then record one stable final all gate. Exact coordinator current40 key list was not found in allowed docs; do not silently claim an invented list is that profile.
- Do not touch string/detokenizer/encoding owners, shared tools/maps/default/ABI tables, or global/current locale.
