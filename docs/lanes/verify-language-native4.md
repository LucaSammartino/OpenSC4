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
- No game requests outstanding. No native measurements or coverage credit.
- Do not touch string/detokenizer/encoding owners, shared tools/maps/default/ABI tables, or global/current locale.
