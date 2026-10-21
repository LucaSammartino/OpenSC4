| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-view3dwin4: cSC4View3DWin window spec (view3d-window.md) | `re-view3dwin4` / (RE spec lane) | ready (round 2): §3.1 per-cheat contracts (VW1 answered), §6b AddGUI/RemoveGUI structure, VW6 filed, tests VW-T5/T6; docs-only, no gate, no game runs | 1. Spec the input handlers (ProcessKeyDown/Up, mouse). 2. VW6: AddGUI tail 0x007EEDB0-0x007EF1B6. 3. VW2-VW5 unchanged. 4. 8 ABI rows stay proposals (build-runs/abi-rows-re-view3dwin4.tsv). |

## Handoff (round 3)

Goal: round 3/3 of the cSC4View3DWin spec: input handlers, then VW6 AddGUI tail 0x007EEDB0-0x007EF1B6 and per-panel
extras 0x007EE39B-0x007EE610; VW2-5 only if budget allows. Owner stays partial.
Done: §6c input handlers (keys, character, all mouse slots), §6 key-helper offset fixed to +0x120, tests VW-T7..T9
(commit: see git log, "§6c").
Next: VW6 AddGUI tail (re_query.py asm 0x007EEDB0 pages), write into §6b steps 6-7, answer VW6 if complete.
Facts: mouse/char handlers this = obj+0xC; stack +0x134, persistent +0x140; build-runs/casm.sh ADDR SKIP MAX prints
compact asm. REA not used.

## Round 2 report

What changed (docs/specs/view3d-window.md):

- §3.1 `ProcessCheatCode` (`0x007F3510`), walked on the instructions: call path (window message `0x230E27AC` →
  `cRZCmdLine` from data2 → `this − 0xC`; result = message consumed), per-cheat table for all 13 IDs with argument
  rules and return values; the two filter cheats (`TastyZots` toggles realistic-filter `+0xD`, `Zoneria` toggles `+0xC`
  and clears `+0xE`, both return false *[quirk]*); `SizeOf` clamp [0.9, 64] and the camera control store at
  `0x007CD6E0` (bounds test the old value); `RenderProp`/`RP` argument forms and the usage text through an empty slot
  45 stub; `Recorder` = `AddWinAnimationRecorder` (child `0xA802B4EB`, `<UserData>RecordedAnimations\`).
- VW1 answered DIFFERS (round-1 notes said the byte-flag cheats return IDs; they return true; 0.9/64 are SizeOf's
  clamp). Moved to questions/answered/view3d-window.md.
- §6b `AddGUI`/`RemoveGUI`: caller pairing (DoPreCityInit end / Shutdown), main panel, the five tab panels with
  script instances and window IDs, per-panel lists and generator slots, text fields, news ticker, four more script
  windows and their winprocs, and the RemoveGUI teardown order. VW6 filed for the AddGUI tail and the fields RemoveGUI
  releases but AddGUI's first part does not create.
- Tests VW-T5 (SizeOf), VW-T6 (filter cheats, TerrainQuery).
- Commits: 01f83a9c (§3.1, VW1), ee4cf05a (§6b, VW6), this one (tests, report).

REA not used: all reads from tools/re_query.py, decomp/ and exe data bytes (build-runs/peread.py).
Not covered: input handlers (ProcessKeyDown/Up, mouse), VW2-VW6. ABI rows not applied (task rule).

## Round 1 report

What changed:

- New spec `docs/specs/view3d-window.md` for owner `cSC4View3DWin` (decomp file: 149 functions). It covers what
  ui-windows.md §3-4 and view3d.md §5 did not: §1 the four interface sub-objects (`+0` cISC4View3DWin 0x00ABCE40,
  `+4` cIGZMessageTarget2 0x00ABCE2C, `+8` window message filter IID 0xC6AE7085 0x00ABCE1C, `+0xC` cIGZWin
  0x00ABCBC0, 151 slots with the 17 overridden slots listed), QueryInterface's exact five-IID set, one shared refcount;
  §2 constructor order and defaults, the 33-type notification set at 0x00ABC070; §3 Init (command-name table
  0x00B0DD98, 13 cheat registrations with ID/text pairs read on the instruction order, three conditional), §4
  Shutdown (unregisters all 13 cheats unconditionally, quirk), §5 notification routing table (command messages to
  ProcessCommand first, then 25 types), §6 window DoMessage routing, focus-loss scroll stop, link-result quirk,
  the filter; §6a DoPreCityInit/DoPostCityInit (camera control at +0xF0 with start focus (512,0,512), renderer at
  +0xF4, helper at +0xEC, model maker at +0xF8). Four conformance tests VW-T1..T4.
- Offsets are absolute and checked on the instructions: the decomp's `this` is obj+4 in 0x007F53C0 and obj+0xC
  in 0x007F3E50, so its field names there are shifted.
- Questions: docs/specs/questions/view3d-window.md VW1-VW5; QUESTIONS.md reindexed.
- Proposed ABI rows: build-runs/abi-rows-re-view3dwin4.tsv (vtables.tsv format, 8 rows: 0x00ABCE2C and 0x00ABCE1C,
  not in docs/abi).
- REA was unavailable ("No app is open"); all reads came from tools/re_query.py, decomp/ and raw image bytes.

Not covered: per-cheat effects (VW1), AddGUI/RemoveGUI, the catalog item/building list builders, the input
handlers' detail (city-tools-input.md has part), cost indicator and cursor text.
