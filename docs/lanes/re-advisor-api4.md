| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-advisor-api4: advisors AD7 (Lua registration order), AD8 (GetRandom/DoLink), AD9 (cheat text payload), AD10 (reload calls, directories) | `re-advisor-api4` / (RE lane) | in progress | answer AD7-AD10 |

## Handoff
- Goal: answer AD7-AD10 in docs/specs/advisors.md (sections 8.2, 12.1, 5.2, 4.2), archive them.
- Done: worktree created; AD7 written in §8.2 (register order table).
- Next: write AD8 (§12.1 slot 9 caller = random_chance 0x40A810; DoLink 0x41B080 parse), AD9 (§5.2 GetVoid2 +0x3C cIGZString*), AD10 (§4.2 dirs: +0xD4 GetPluginDirectory(-1), +0xDC GetSkuSpecificDirectory; cdecl), archive.
