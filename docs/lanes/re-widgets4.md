| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widgets4 (widget toolkit spec: cGZWin base window + cRZWinKeyAccelerator) | `re-widgets4` / (RE lane) | in progress (round 3): widget-toolkit.md section 17 (draw rectangles, child_private_buffers, invalidation, converters, params, EnumChildren, SortChildren, placement, colours) | 1. cRZWinKeyAcceleratorRes parser 0x00984587. 2. remaining cGZWin gaps in section 12. |

## Handoff (round 3)

- Goal: finish the section 12 cGZWin gaps, then cRZWinKeyAcceleratorRes parser 0x00984587.
- Done: section 17 (17.1 draw/origin/blit rectangles with worked example, 17.2 child_private_buffers, 17.3
  invalidation, 17.4 converters/forwarders, 17.5 params, 17.6 EnumChildren/SortChildren, 17.7 Center/Fit,
  17.8 colours/transparency); quirks 11-14; tests 17-21; section 2 rows +0x24/+0x34/+0xB8; section 12 row updated.
- Next: parser 0x00984587 (only if budget allows), then final report + merge-queue file.
- Facts: all evidence from re_query.py asm; REA not used. Draw-system service 0x4B2EA3B3 cached at 0x00BAC060/05C.

## Report (round 2)

What changed (all in docs/specs, no engine/ABI change):
- WT4 answered DIFFERS: SetFlag 0x0099DB6B has a single exit (every branch jumps to 0x0099DCD9, MOV EAX,[ESI+0xC8];
  RET 8) so it returns the whole flag word; ui-windows.md section 2.2 row 68 corrected (only that row); entry moved to
  questions/answered/widget-toolkit.md; QUESTIONS.md regenerated. Commit 00672ce4.
- widget-toolkit.md section 14 painting (d4923d75): Plot = PlotComposite && PlotPresent then manager deferred-plot
  flush when drawing to the screen; PlotComposite steps (fade gate, paint filters type 0x16, clean-window shortcut via
  +0x70 dirty byte, draw-context setup by slot, GZPaint, back-to-front child walk with overlay 0x8000000 deferral and
  rectangle-overlap propagation); PlotPresent destinations (parent buffer with +0xB8 rect, or screen with absolute rect
  when 0x4000000), colour/alpha packing verified at 0x0099C63D; PrivateBuffer create/destroy rules; SetBufferToDrawTo;
  cGZFadeTmpWin fade-out (negative second period +0x98), which corrects the round-1 claim "no fade-out" (section 8.2,
  quirk 8 rewritten). Manager slots 7/8 (deferred plot queue at +0x184/+0x188) measured.
- Sections 15-16 (3d73228a): ProcessCursorMessage routing (front-to-back, 0x200000 skip, disabled children get only
  moves, PullToFront+SetFocus on types 7/8/9, in-place ScreenToWindowCoordinates, UpdateCursor on moves), manager slot
  26 SetMouseWindow (enter 19 / exit 20 sent synchronously), UpdateCursor (+0x74 added to layout), key equivalents
  (slots 78-82, exact packing, 0/1 return). Conformance tests 11-16 added.

Evidence: re_query.py asm (PlotComposite fully paged, PrivateBuffer, PlotPresent colour block, ProcessCursorMessage,
StealBuffer, manager slots 7/8/26) plus decomp/cGZWin.c; REA not queried.

Owner coverage (honest):
- cRZWinKeyAccelerator: specified 16/16 (unchanged); cRZWinKeyAcceleratorRes parser still open.
- cGZWin: specified for layout, flags, children, geometry, hit test, dispatch, lifecycle, fade in/out, painting, mouse
  routing, key equivalents. NOT complete: gaps enumerated in widget-toolkit.md section 12 (draw-area recursion and the
  +0x24/+0x34/+0xB8 rectangles, recalc_blt_area, child_private_buffers, transparency helpers, Invalidate*,
  coordinate converters, PullToFront, SetFocus, EnumChildren, params, SortChildren, Center/Fit, colour conversion,
  draw-context/buffer slot meanings).
- cGZWin* (widgets), cGZLegacyControl, cGZUIScript*, cHTML*, cGZGraph, cGZ*Graph, cSC4ScriptedWindowCOMDirector and
  the rest: not specified.
Questions: WT4 answered; WT1-WT3, WT5, WT6 open; none new (a WT7 draft was resolved from bytes at 0x0099C2BC before filing).

## Report (round 1)

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
