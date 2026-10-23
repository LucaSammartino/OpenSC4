| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-view3dwin: the city view's fields, tools, commands, panels, menus and status displays, folded into view3d-window.md (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `f01d1ce4`, after re-powerpoles) | **ready** (docs and RE tables only). view3d-window.md §6d–§6m added (fields, tool stack, scrolling and views, all 104 commands, panels, buttons, menus and catalog popups, status displays, the save record's body and global triggers, helpers, quirks) and §2, §3, §3.1, §6, §6.3, §6a–§6c completed; VW2–VW5 and OM10 answered, VW6 narrowed, VW7–VW8 filed; ui-windows.md, save-records.md and prop-developer.md corrected; the `cSC4View3DWin` coverage row now cites its specs; 171 names (20 new, 151 renamed). Specified coverage +68,620 B / +202 functions | VW7 (message `0x470BD214`), VW8 (the objects at `+0x184`, `+0x188`, `+0x410`), VW6 (still open parts) |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`), read
with the local disassembler, the Ghidra export (switch trees resolved with build-runs/re-tools/switchwalk.py and
switchcases.py) and the SDK's `cIGZGraphicSystem` slot order. Written on 2026-10-21 as a separate `city-view.md` and
held back when `main` gained view3d-window.md (re-view3dwin4); this is that work folded into view3d-window.md. Where
the two disagreed upstream was right once (`ProcessKeyDown` also returns true when a persistent control takes the
key, `0x007E67A6`; this lane's §3 had it wrong and the fold keeps upstream's §6c), and this lane's text otherwise.

### What changed

- **view3d-window.md**, new sections: §6d the fields (object offsets, the meanings §2 marked inferred); §6e the view
  input control stack (slots 25–30, the change message); §6f scrolling (`RecalcScrollVelocity` and its angle table,
  `SetScrolling`, `ScrollStop`, the per-paint step, `DoScrollBegin/End`), zoom and rotation, the view filter and level,
  the hidden pause, contours and terrain, the tuning; §6g every one of the 104 commands of the table `0x00B0DD98` and
  what `ProcessCommand` does with it; §6h the message filter's branches, `DoPanelCommandGUI`, `ShowPanel`,
  `HidePanel`, `ShowMayorSubPanel`, `MinimizeUI`, establishing and obliterating; §6i `HandleButtonActivated`, every
  menu, the catalog popup `0x007EAEB0`, `InvokeCatalogItemCommand`, `InvokeDisaster`; §6j the date, funds, population,
  speed buttons, clocks, light, mayor rating bar, alert border, cursor text, cost indicator, mission panel; §6k the
  View3D record's body (version 4, four references, the trigger pairs) and the global effect triggers; §6l helpers;
  §6m quirks; tests VW-T10–T18.
- **view3d-window.md**, completed: §2's defaults table meanings; §3 step 2 (104 records) and step 8 (the key
  accelerator); §3.1 the meaning of `WatchMeMove` and `You don't deserve it`, `SetRenderProperties`, and the realistic
  filter's flags as occupant-manager.md §13.2.3 now gives them; §6 the advice links; §6.3 the filter branches; §6a
  the S3D camera at `+0xEC`, the tick manager, model maker, tuning and `AddGUI`, the established/obliterated branch
  (city slot 22), the RCI graphs, gauges, the city-situations control; §6b the control panel's x; §6c the graphic
  system (`0x00B43C9C`, service `0xC416025C`, slot 12 `IsFullScreenMode`).
- **ui-windows.md**: the `cIGZWin` vtable is `0x00ABCBC0`; `0x007EAEB0` is the catalog popup and has callers (quirk
  12); the View3D record's body and version point at view3d-window.md §6k.
- **save-records.md** Q4: the View3D part answered. **prop-developer.md** §4.10: the trigger message is sent, not
  posted.
- **occupant-manager.md** §13.2.3: the cheats `Zoneria` and `TastyZots` (OM10, which re-occfilters filed with a wrong
  guess).

### Names and coverage

- re/symbols/city-view.tsv: 171 names (20 new; 151 replace auto names, among them `lib::__allmul` `0x009F0D10` and
  `lib::__alldiv` `0x009EFEB0`, which the tables had wrong, and wrong Mac matches). The `names_windows.tsv` delta
  (−151 +171) is identical to `merge_names.py`'s; no non-auto upstream name is replaced.
- tools/coverage-map.tsv: the `cSC4View3DWin` row cites view3d-window.md, ui-windows.md and view3d.md (it had none).

### Measured

`tools/coverage.py` on the local (Steam) export, measurement only: after re-powerpoles 3,237,844 bytes in 14,650
functions; with this lane 3,306,464 bytes in 14,852 functions (+68,620 B, +202 functions; denominator unchanged).
`docs/coverage-report.md` was not regenerated (the tool accepts only the GOG exe).

### Questions

- Answered and moved to questions/answered/view3d-window.md: VW2 (the key accelerator), VW3 (104 commands), VW4 (the
  filter branches), VW5 (the S3D camera; city slot 22). OM10 (occupant-manager) answered from §3.1.
- Narrowed: VW6 (`0x00B43C9C` is the graphic system). Filed: VW7 (`0x470BD214`), VW8 (`+0x184`, `+0x188`, `+0x410`).
