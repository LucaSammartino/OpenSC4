| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-citytools4: city tools and input spec (view input control base, place-lot tool) | `re-citytools4` / (RE lane) | ready: spec docs/specs/city-tools-input.md (commit 02d2712a); questions CT1-CT5 filed; no ABI, no game runs | 1. specify CT5 (PlaceLot, CanPlaceLot, facing helpers, SetVisualUserFeedback) to claim cSC4ViewInputControlPlaceLot; 2. then Dispatch / Zone / Demolish tools on the same base; 3. Query (76565 bytes) needs its own lane |

## Report

What changed:
- New docs/specs/city-tools-input.md. §2 `cSC4ViewInputControl` (vtable `0x00AA6658`, IID `0xC6BB8345`): layout
  `+0x04..+0x20`, all 30 slots with stub behaviours, Init/Shutdown order, the two SetCursor forms, capture,
  IsOnTop, EndInput (removes the stack top), PlayResultSound `(1, id, 0, 0, 0)` on `cIGZSndSys` slot 11.
  §3 `cSC4ViewInputControlPlaceLot` (size `0x158`, vtables `0x00A90738` / `0x00A90728`): constructor defaults,
  creators, Init (ID `0x88F154FB`, cursor `0x88E70B76`, six message subscriptions, lot/building pick, empty set
  -> Shutdown + false), Shutdown, DoMessage table, rotation keys `0x6A935CD5` / `0x6A935CD1`, left click
  (PlaceLot, world rect via `_ftol2` low dword, 1000-plop limit, failure sound `(0xEA451903, 0xAA62C9D5)`),
  mouse move (three facing paths, cost indicator, 1e-4 change test), mouse exit. Quirks §4, tests §6.
- docs/specs/questions/city-tools-input.md: CT1-CT5; QUESTIONS.md regenerated.

Sources: export (re_query.py func/asm) and decomp/; numeric parts (the 2^32 bias, the 1e-4 epsilon at `0x00A90688`,
the stub bodies) checked on the instructions. REA was not used.

Proposed coverage-map row for the coordinator (exact owner, all 17 functions, 592 bytes), to go ABOVE the
`cSC4ViewInputControl*` glob row 358 of tools/coverage-map.tsv:
`cSC4ViewInputControl	city tools and input	ui	city-tools-input.md	-	-	-`
Not claimed: `cSC4ViewInputControlPlaceLot` (21780 bytes / 42 functions), only lifecycle and input specified;
its bodies are CT5.

Measured: subsystem 182466 bytes uncovered; per owner (coverage.py --owners): Query 76565/216, PlaceLot 21780/42,
PlayerDrive 12400/40, NetworkToolUI 8005/41, MySimChangeJob 7233/21, Zone 6929/22, Dispatch 3367/16, base 592/17.
