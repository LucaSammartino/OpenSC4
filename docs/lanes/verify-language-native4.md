| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-language-native4 (native utility verification) | `Codex` / `verify-language-native4` (engine lane) | in progress | Establish published native acquisition, typed ABI, storage and lifetime within 15 calls. |

## Handoff
- Goal: compare production language utility core/CAPI against independently owned original utilities for all IDs 0..0x2d.
- Worktree reused at detached HEAD; initial status clean; no lane commits yet.
- Baseline full build deliberately skipped per task-specific instruction; focused checks follow code, one stable final full gate.
- Safe path established before call 15: SDK cIGZLanguageManager service 0x441e5070 / IID 0x0054b7d5; GetNewLanguageUtility slot 39 owns count 1; SDK utility slots 0..29; setters slots 31/32 from published ABI, void per spec. Existing version-guarded fc4::GameString + B allocator owns native output buffers (docs/abi/allocators.md Strings); no new executable bindings.
- Next: implement a private CAPI transport to the actual LanguageUtility, and a plugin probe using the published native service and existing storage. Preserve LU1-LU3/SL1 gaps.
- No game requests outstanding. No native measurements or coverage credit.
- Do not touch string/detokenizer/encoding owners, shared tools/maps/default/ABI tables, or global/current locale.
