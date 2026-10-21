| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-advisor-api4: advisors AD7 (Lua registration order), AD8 (GetRandom/DoLink), AD9 (cheat text payload), AD10 (reload calls, directories) | `re-advisor-api4` / (RE lane) | ready, docs only, no ABI change; AD7-AD10 all DIFFERS from the parked wrapper guesses, answered and archived; AD11 filed | 1. glue-advisors: apply advisors.md §8.2 order, §12.1 slot 9/11 contracts, §5.2 GetVoid2 payload, §4.2 directories + cdecl native calls. 2. runtime read of GetSkuSpecificDirectory (AD11). |

## Report
Spec-only lane (static analysis via tools/re_query.py and decomp/; REA not needed). Commits f7d5c6b0 (AD7), 44d3462c (AD8-AD10, AD11).

- AD7 (advisors.md §8.2): vector 0x00B207CC..D4, simulation collector first (CRT table 0x00B07030 before 0x00B07034), full 47-entry order table; UI order ends window_query, window_graph. `_tf_` = print after register_api(true), window_graph (0x0040C890) after register_api(false). Conformance 23.
- AD8 (§12.1): slot 9 only caller game.random_chance 0x0040A810 → non-virtual cRZRandom::RandomDoubleRangeUniform on the result; state u32 at +4 (lazy clock() seed, ×0x278DDE6D). Null DIFFERS (fault). Slot 11 DoLink 0x0041B080: "%08x%08x" parse, advice slot 22 forward, else Lua DoBuffer from char 16 (ID 0x1D) or 8 (other fallback) inside Push/PopContextAdvice; returns true (false only on parse failure). Conformance 24, 25.
- AD9 (§5.2): cheat text = message-2 data2 (GetVoid2 +0x3C) borrowed cIGZString*, whole typed line, set by cGZCheatCodeManager::SendCheatNotifications 0x00920ECE; no null test. Conformance 26.
- AD10 (§4.2, lua-host.md §7.3 and dependency table): UpdateResourcesFromDirectory, LoadPackageScripts are cdecl caller-pop (8/12 bytes), int return ignored; _dofile_from_resource is a lua_CFunction needing the game cSCLua state. Directories: cISC4App +0xD4 GetPluginDirectory(out,-1) then +0xDC GetSkuSpecificDirectory(out); data directory not used; lua-host.md slot names corrected. Conformance 27.
- AD11 filed: runtime value of GetSkuSpecificDirectory (app +0xC4 string not filled in PreFrameWorkInit).
- No engine/, tools/ or docs/abi edits; no ABI rows proposed.
