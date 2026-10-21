| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widgets4 (widget toolkit spec: cGZWin base window + cRZWinKeyAccelerator) | `re-widgets4` / (RE lane) | ready: new docs/specs/widget-toolkit.md (sections 1-13), questions WT1-WT6 filed; commits 20b38bdb, 17919733; no engine/ABI change; REA NO-GO (no binary open), evidence from decomp + re_query asm + raw image reads | 1. cGZWin painting slice (PlotComposite 0x0099E62D, PlotPresent, PrivateBuffer 0x0099EA70). 2. cursors (ProcessCursorMessage 0x0099D40B), key equivalents. 3. cRZWinKeyAcceleratorRes text parser 0x00984587. 4. cGZWinGen, then widgets. |

## Report

What changed:
- `docs/specs/widget-toolkit.md` (new): `cGZWin` identity (vtable 0x00ADC8D8, ctors 0x0099D938/0x0099DA5A, dtor
  0x0099E1A2), object layout +0x00..+0xD8, flag word (default 0x8903, bit meanings, exact-mask side effects of
  SetFlag), copy-on-write child list, add/remove/delete/reparent, z-order, geometry (area vs absolute area,
  half-open rect test 0x00664C60), point tests and hit-test recursion, message filters, DoMessage jump table
  (0x0099CEF9, types 1-20 to slots), Send/Post, Init/Shutdown/destructor, fade-in math (x87 double, constants
  verified from the image), quirks, 10 conformance tests; section 13 `cRZWinKeyAccelerator` (all 16 slots, key-code
  packing, wiring into cGZWin).
- `docs/specs/questions/widget-toolkit.md`: WT1-WT6; `docs/specs/QUESTIONS.md` regenerated.

Findings worth noting:
- ui-windows.md section 2.2 says SetFlag returns 0/1; measured it returns the whole flag word (0x0099DCD9). Filed WT4.
- ui-windows.md section 2.3 inferred window +4 is the window manager: confirmed by the constructor (RZ::WM() stored at +4).
- `questions.py check` reports a pre-existing problem unrelated to this lane (questions/occupants-props.md has no spec).

Owner coverage (honest):
- `cRZWinKeyAccelerator`: specified (16/16 functions). Pattern `cRZWinKeyAccelerator*` also matches
  `cRZWinKeyAcceleratorRes` (4 functions, parser not specified): partial for the pattern.
- `cGZWin` class: specified for layout, flags, children, geometry, hit testing, dispatch, lifecycle, fade,
  accelerator wiring; NOT painting, private buffer, cursors, key equivalents, SortChildren, colour conversion.
  The map pattern `cGZWin*` covers all widgets and is not specified as a whole.
- Not specified: cGZLegacyControl, cGZUIScript*, cHTML*, cGZGraph, cGZ*Graph, fn:unscript, fn:script_tag_*,
  fn:GetGZWinFont, cSC4ScriptedWindowCOMDirector, cRZCursorManager, cGZClipboardManager*, cGZCheatCodeManager.
- Proposed coverage-map rows (coordinator's call): a `cRZWinKeyAccelerator` row ahead of `cRZWinKeyAccelerator*`
  with spec docs/specs/widget-toolkit.md; a `cGZWin` exact row ahead of `cGZWin*` marked partial (spec
  docs/specs/widget-toolkit.md).
