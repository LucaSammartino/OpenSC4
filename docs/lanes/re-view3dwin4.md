| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-view3dwin4: cSC4View3DWin window spec (view3d-window.md) | `re-view3dwin4` / (RE spec lane) | ready: new docs/specs/view3d-window.md §1-§6a, questions VW1-VW5 filed, ABI rows proposed in build-runs/abi-rows-re-view3dwin4.tsv (8 rows); docs-only, no gate, no game runs | 1. Answer VW1 (per-cheat effects of ProcessCheatCode 0x007F3510). 2. Spec AddGUI/RemoveGUI (0x007EDEB0/0x007EF200) and the input handlers. 3. Land the 8 ABI rows for vtables 0x00ABCE2C/0x00ABCE1C. |

## Report

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
