| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widgets4 (widget toolkit spec: cGZWin base window + cRZWinKeyAccelerator) | `re-widgets4` / (RE lane) | ready (round 3, final): widget-toolkit.md section 17 + 4.3 rows (draw rectangles, child_private_buffers, invalidation, converters, params, EnumChildren, SortChildren, placement, colours, reordering steps); commits 5559eedf, c4b9b64f; no engine/ABI change; REA not used | 1. cRZWinKeyAcceleratorRes::SerialRead 0x00984587 (2579 bytes, not started). 2. cGZWin slot meanings of draw context/buffer/GS/WCM, create_private_buffer internals, +0xA0..+0xA4 (WT2). 3. cGZWinGen, then widgets. |

## Report (round 3, final)

What changed (docs/specs/widget-toolkit.md only; no engine/ABI change):
- 5559eedf: section 17. 17.1 the three rectangles: draw area +0x24, draw origin rectangle +0x34 (feeds draw-context
  slots 8/6), blit rectangle +0xB8 (recalc_blt_area 0x0099C7BA); SetAreaToDrawTo 0x0099CF6A two cases incl. the
  once-cached service 0x4B2EA3B3 that moves the draw area to (0,0,W,H), overlay absolute-position rule, worked example;
  the two Recursive walkers. 17.2 child_private_buffers 0x0099D645 table. 17.3 InvalidateSelf/SelfAndParents/
  AllParents (overlay stops). 17.4 converters and forwarders. 17.5 GetParam (AddRefs), SetParam (release before
  AddRef; null removes), EnumParams. 17.6 EnumChildren (empty -> false, IID 0x22BA0121 path), SortChildren
  (restarting stable insertion sort, default unsigned-ID comparator 0x0099BC0E, flag 0x800 windows not moved unless
  moveAll). 17.7 CenterWindowInRect (truncating /2) and FitRectToWindow (asymmetric clamp) with examples. 17.8 colour
  conversion and the transparency helpers. Section 2 rows +0x24/+0x34/+0xB8; quirks 11-14; tests 17-21.
- c4b9b64f: 4.3 rows ChildStepFront/Back, ChildMoveRelative, IsWinInChildChain; MoveRelativeTo argument order;
  GZOnSetFocus/GZOnMouseWheel defaults; section 12 coverage row.
Painting, cursor, key-equivalent and WT4 text from rounds 1-2 untouched.

Evidence: re_query.py asm of every cited function (addresses in the spec); REA not queried.

Owner coverage (honest): cGZWin specified for everything listed in section 12 except: draw-context/buffer/graphics-
system/colour-manager slot meanings (cited by number), create_private_buffer internals, +0xA0..+0xA4 (WT2), open
questions WT1-WT3, WT5, WT6. So cGZWin is NOT complete. cRZWinKeyAccelerator 16/16; cRZWinKeyAcceleratorRes parser
SerialRead 0x00984587 (2579 bytes) not started. cGZWin* widgets, cGZLegacyControl, cGZUIScript*, cHTML*, cGZGraph,
cGZ*Graph, cSC4ScriptedWindowCOMDirector: not specified.
Questions: none new; none answered this round.

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
