| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-terrain3d (TR21 terrain 3D view contract: slot behaviours, change messages, draw statics, the replacement checklist, into terrain.md §15.5) | `re-terrain3d` / (docs lane) worktree `.claude/worktrees/re-terrain3d` | **done, tip `751172ad`** (7 commits, all docs/tools): terrain.md §15.5 completed — §15.5.5 slot table with call sites, §15.5.8 change-message handlers + rebuild-vs-live rule, §15.5.9 draw-time statics/buffers + `cSTEVertBufExtn`, §15.5.10 every slot's behaviour with pop/ret channels, §15.5.11 the 17-point replacement contract; §15.5.2 field meanings; quirks 22–25; TR21 marked completed; questions **TR28–TR32** filed (5 open); `tools/coverage-map.tsv` points `cSTETerrainView3D` at terrain.md; ABI row proposals in `build-runs/abi-rows-re-terrain3d-{interfaces,vtables}.tsv` (34 rows, build-runs is gitignored — copy them into docs/abi/ at merge time if accepted) | - |

## Report (re-terrain3d, 2026-10-20)

**Goal.** Fill `docs/specs/terrain.md` §15.5 (the `cSTETerrainView3D`, class `0xC9B84E10`, contract) so the
`enable = terrain` lane knows what the 3D view needs (HANDOFF open item 4).

**What was already there.** re-answers30 (2026-10-16) had written §15.5.1–§15.5.7: creation by `DoPostCityInit`,
object layout, the five vtables with return channels, `Init`'s contract, the two unchecked globals, slot 66 and
the game's init sequence.

**What this lane added** (verified against the Windows instruction bytes; every code address cited resolves —
checked 193 `.text` addresses in the new sections with `re_query.py func`, 0 unresolved):

1. **§15.5.5 extension** — the full slot table the view calls on the terrain, each with its call site:
   1, 7, 8, 42 (Init), 21/22 (Update, x87 in st0), 29 (both change-message handlers), 42 (DrawTerrainLayers,
   each frame), 67 (DrawTerrainLayers' once-per-session lighting hook with `V+0x34`, gated by `V+0x31`),
   68 (SetDisplayGridFlag's write-back), 70 (`0x00759C7C`) and 76 (`0x00759C57`) (DrawPlumbingInfo, per cell),
   83 (DrawWaterLayers). Plus the water-array direct read: `Pick` (`0x0075A357`) and `FUN_00756B00`
   (`0x00756B10`) dereference `view+0x24` → **water `+0x18`** (0x24-byte records, depth at +0x00).
2. **§15.5.8 (new)** — the change-message handlers of `DoMessage` (`FUN_00757560`): all six subscriptions, the
   rect unpacking (`FUN_007523F0`, the same `(x0<<16)+x1` packing §8.1 posts), the heights copy
   (`FUN_007569F0`), the water refill (`FUN_00756B00`: surface = `depth > 0 ? altitude + depth : sea level`),
   the `view+0xB8` dirty byte, and the **rebuild-vs-live rule**: the view is a pull-through cache; terrain edits
   reach it only through the `0x278E6003`/`0x678E6E3A` posts; only slots 21/22/42 are re-read per frame; the
   index buffers are built once at `Init` (`FUN_00755FB0` over the whole grid) and per-cell afterwards.
3. **§15.5.9 (new)** — the draw-time statics table (`0x00B4C744..0x00B4C75C` geometry, `0x00B4C6F4/F8` index
   buffers, `0x00B4C6B8` level map, the edge-layer arrays, the texture-binding factory `0x00B4C704`, the view's
   own ini properties `0x29AB15C0`–`C7`) and `cSTEVertBufExtn` (gated on the preference byte
   `[[0x00B43CC4]+0xC]+0x33C`; a replacement host can leave it unsupported).
4. **§15.5.10 (new)** — every `0x00AB4410` slot's behaviour with pop sizes and return channels, including:
   QueryInterface's five answers plus the **unnamed extra IID `0xC989F960`** (answered with the object base,
   asked by `cSC43DRender::DoMessage` `0x007CB1C1`); the five `MarkSelected` wrappers (cell/rect/point/vector/
   span, textured op-2 vs untextured); `SetAutomaticCellCursorUpdateStatus` returning the old value;
   `SetWaterGlareStatus`'s replace-vs-union rect bookkeeping; `DrawTerrainQuad`/`DrawTexturedTerrainQuad`'s
   direct flag reads and index-buffer choice; `LevelCell`/`RemoveCellLevelling`'s direct writes
   (`or/and byte [+0x22], 0x60/0x9F`); `GetOverlayManager` without bounds check; `HandleCheatCommand`
   (`"level x z amount"` → slot 16, `"unlevel x z"` → slot 17); the grid flag living in the static
   `0x00B4C741`; and the `0x00AB43E8` save/load pair (the saved byte of §13.2 is the grid-display flag).
5. **§15.5.11 (new)** — the 17-point contract checklist for a replacement terrain/water: creation and identity,
   the eleven slot dependencies with the stage each is needed at, the direct field reads/writes (`T+0x6C` live
   and writable, record `+0x16` texture id, `W+0x18` live), the message contract with the §8.1 packing, the
   unchecked globals, and the ordering rule.
6. **§15.5.2** — the layout table rows enriched with the verified field meanings (`+0x28/+0x2C` zone and lot
   managers, `+0x31` first-draw gate, `+0x34` lighting-hook argument, `+0x38` draw-context cache, `+0x3C/0x3D/
   0x40/0x48/0x49/0x4C` glare and rect bookkeeping, `+0x44` the `0xC9DA96EA` data2 sink).
7. **§16** — quirks 22–25 (the static grid flag shared process-wide, the setter-that-returns-old-value, the
   unbounded `GetOverlayManager`, the extra IID).
8. **`tools/coverage-map.tsv`** — `cSTETerrainView3D` now points at `terrain.md` (was unspecified; the
   regenerated report counts it under the renderer group once merged).

**Questions filed** (`docs/specs/questions/terrain.md`, 5 open; `python tools/questions.py index` + `check` ran
clean): **TR28** the extra IID `0xC989F960`; **TR29** message `0xC9DA96EA`'s identity and `data2` meaning;
**TR30** the city-edge layer-height refresh rule (`FUN_00755B60`); **TR31** `LevelCell`'s ST0 return channel;
**TR32** the `StaticInit` functor pair (`0xC9EFB727`/`0x69EFB6F8`) and its level→colour LUT.

**ABI rows proposed** (not written into docs/abi; per the lane rules they live in
`build-runs/abi-rows-re-terrain3d-interfaces.tsv` (20 rows: the `0x00AB4410` slots 0, 3–21 with behaviour
notes) and `build-runs/abi-rows-re-terrain3d-vtables.tsv` (14 rows: the previously untabulated
`0x00AB4400` 8-slot message-target table and the `0x00AB43E8` 6-slot save table). `build-runs/` is gitignored,
so the merge agent should copy them from the worktree if accepted.

**Not done (deliberately):** no engine/ or tools/ edits beyond the one-line coverage-map row; no game runs
(docs lane); `docs/coverage-report.md` left to the merge train (running `tools/coverage.py` from the worktree
writes the main checkout's copy by default — the coordinator regenerates it after the batch lands).

**Next steps:** none for this lane. The `enable = terrain` glue lane can now work from §15.5.11; the questions
TR28–TR32 are follow-up RE work if the ABI tables need the names.