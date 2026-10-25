# The drag-and-drop manager (`cGZWinDragAndDropManager`) (behaviour spec)

Scope: the `cGZWinDragAndDropManager` owner as tools/coverage.py assigns it on `bb641878` (48 functions, 3,518
bytes; inventory in §6). The manager is a service and a window at once: it holds the current drag (a `DragData`
record), turns presses on registered "drop source proxy" windows into drags, moves an image of the source under the
pointer, finds the window under it and sends start, enter, over, leave, drop and cancel messages to the source and
the target.

Related:
- widget-comdirector.md §2: class `0xAC0874D8` → `0x0099897A` (`0x108` bytes, the factory returns `+8`);
- widget-toolkit.md: the `cGZWin` it embeds (filters, slots 116/117; `WindowToWindowCoordinates`, slot 61; message
  types §7.3), and widget-winmgr.md (capture, `GetCursorPos`, `IsWindowValid`).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the proxy filter, the drag threshold, both proxy
registration slots and the constructor were checked on the instructions. The vtables `0x00AE2018`
(`cIGZWinDragAndDropManager`, 14 slots), `0x00AE2008` (filter), `0x00AE1FD4` (system service), `0x00AE1FBC` and
`0x00AE1D60` (the window) were read. Written 2026-10-24 (re-gzdragdrop7).

Conventions: "slot n" is a `cIGZWinDragAndDropManager` slot, "window slot n" a `cIGZWin` slot of the embedded window
`W` (`+0x24`). `D` is the drag record. "Send(m)" is §3's message to the target and the source.

## 1. Identity and fields *[compat]*

- **Interfaces** (`QueryInterface` `0x009CC235`): `0x4C0874D7` → itself; `0xC6AE7085` (window procedure) → `+4`;
  `0x287FB697` (system service, ID `0x0C0874C6`, priorities `0x1312CF`) → `+8`; `0x22BA0121` (`cIGZWin`) → `+0x24`;
  else false. A COM base table is at `+0xC`. Constructor `0x009CCF9C`; deleting destructor `0x009CD196`; thunks
  `0x009CD032`…`0x009CD0A2`.
- **Fields**: `+0x24`…`+0xFB` the window `W`; `+0xFC` initialised (byte); `+0xFD` the source's saved
  private-buffer flag; `+0x100` `D` (`0x44` bytes, made by `Init`); `+0x104` the proxy windows (list, counted).

**`D`** (`DragData`, constructor `0x009CC3DD`, destructor `0x009CC53A`): `+0x00` type (0 = no drag), `+0x04` a
`cRZVariant` payload, `+0x18` source window, `+0x1C`/`+0x20` grab point, `+0x24` current target window, `+0x28`/`+0x2C`
current pointer, `+0x30`/`+0x34` drag images (`cIGZBuffer`), `+0x38` which image (1 → `+0x34`, else `+0x30`), `+0x3C`
pending (byte: pressed but not yet moved), `+0x40` button (`0xAC0C7F4A` left, `0x0C0C7F51` right). Windows and images
are counted references.

## 2. The slots *[compat]*

| Slot | Address | Behaviour |
|---|---|---|
| 1 / 2 | `0x009CC225` / shared | the system service's reference count |
| 3 `Init()` | `0x009CC4C8` | once: initialised, `D` allocated. Every call: `W.SetFlag(0x40000, 1)`, `(0x20000, 1)`, `(4, 1)`, `cGZWin::Init(W)` |
| 4 `Shutdown()` | `0x009CCB55` | if initialised: clear it, free `D`, empty the proxy list, `cGZWin::Shutdown(W)` and its result; else false |
| 5 `StartDrag(d, pending)` | `0x009CC905` | false when not initialised. A running drag is cancelled. With `d` of non-zero type: `D := d`, pending := `pending`; not pending → show `W` (§4) at once. True |
| 6 `CancelDrag()` | `0x009CCCAA` | when initialised: a running drag that is not pending gets `Send(0x8C0A1FCC)`; `D` is cleared. `W` releases capture if it holds it. True |
| 7 `Drop()` | `0x009CCD10` | when initialised: `Send(0x8C085F48)`, clear `D`; true (no test for a running drag; capture is kept) |
| 8 `IsDragging()` | `0x009CC287` | `D` exists and its type ≠ 0 |
| 9 `GetDragData()` / 10 `SetDragData(d)` | `0x009CC29D` / `0x009CC5EF` | the pointer / copy into `D` |
| 11 `CreateDragData(out, type, payload, source, button, x, y, image, image2)` | `0x009CC601` | fills `out`: grab point `(x, y)`, target null, pointer 0, which 0, pending 0 |
| 12 `AddDropSourceProxy(id, parent, add)` | `0x009CC2A4` | the descendant with `id` (window slot 35) of `parent`, or of `W`'s main window when `parent` is null → slot 13 `(it, add)`; false when not found |
| 13 `AddDropSourceProxy(window, add)` | `0x009CD0ED` | add: when not listed, the filter `+4` is added to the window (window slot 116) and the window listed (counted); this returns **false**; already listed → true. Remove: unlist and remove the filter (window slot 117); true |

`ClearDragData` (`0x009CC6F3`): type := 0; source, target and both images released.

## 3. Messages to the source and target

`Send(m)` (`0x009CC95C`, `0x009CC7F4`): when the target or the source is set, the message `{m, D, x, y}` (`x, y` the
current pointer) goes to the target, then to the source, each when the window manager still knows it (slot 24) and
it answers `cIGZMessageTarget` (`0xE98B2F57`, its slot 3).

| Type | When |
|---|---|
| `0xAC0FE76D` | the drag starts (the pointer left the threshold) |
| `0xAC085F43` | a new target is entered (sent to the new target) |
| `0xAC085F47` | every move while the drag runs |
| `0x6C085F45` | the old target is left (sent to the old target) |
| `0x8C085F48` | drop |
| `0x8C0A1FCC` | cancel (only after the drag started) |

## 4. The drag *[compat]*

**Proxy filter** (`+4`, `0x009CCB9F`, on any listed window's messages; always returns false, so the window still gets
them):
- left down (7): `CreateDragData(&d, 0xCC0CAE33, the window as a variant, the window, left, x, y, null, null)` and
  `StartDrag(&d, pending)`: a **pending** drag;
- key down (5) or left up (10): `CancelDrag` while dragging (left up always);
- mouse move (13): while dragging, `W` takes the capture.

**`W`'s input** (handlers on `W`):
- **Mouse move** (window slot 138, `0x009CCD4F`): not dragging → `CancelDrag`. While pending: without a source the
  drag stops being pending; otherwise the pointer is converted into the source's coordinates (`WindowToWindow`,
  window slot 61) and when either coordinate differs by more than 3 from its unconverted value *[quirk: the grab point
  is not used]* the drag starts: pending := 0, `Send(0xAC0FE76D)`, the source's private-buffer flag is saved and set
  (window flag `0x10000`), `W` is shown, `W` takes the focus.
  After the start: capture the image once (below); `W.Offset(x − grab x, y − grab y)` (window slot 57) when non-zero;
  the pointer is stored; the target is searched again (below); old target left → `0x6C085F45`, new one entered →
  `0xAC085F43`; then `0xAC085F47`.
- **Left up** / **right up** (136 / 137): while dragging with the matching button: pending → `CancelDrag`, else
  `Drop`.
- **Left or right down** (134, 135), **capture lost** to another window (140) and **Escape** (130): `CancelDrag`
  while dragging. The wheel (139) goes to `cGZWin`.
- **`GZPaint`** (window slot 88, `0x009CC669`): not dragging → `W` is removed from its parent (`0x009CC3C4`). Else the
  chosen image, if any, is drawn at the draw origin at its own size (draw-context slot 38).

**Showing `W`** (`0x009CC740`): with `D` and a main window that does not hold `W` yet: the grab point converted from
the source into the main window's coordinates becomes `W`'s position (window slot 56); its size is the first image's
(buffer slots 9, 10) or 16×16; the main window adds `W` as a child.

**Capturing the image** (`0x009CC999`), once per drag while one image is missing: a new buffer (GZCOM `(0xC470D325,
0x20732180)`) the size of the source's private buffer gets a copy of it (buffer slot 29) and becomes both images;
`W` takes that size. The source's private-buffer flag is restored.

**Target search** (`0x009CC417`): from the source's parent up through its ancestors until a target is found, each
ancestor's children are enumerated (window slot 32 with `0x22BA0121`); a child other than the source and `W` that is
visible (flag 1), has flag `0x8000`, lacks `0x200000` and contains the cursor (window manager `GetCursorPos`; window
slot 62) becomes the target, and its own children are searched in turn, so the deepest such window wins.

## 5. Quirks and cases

Quirks:
1. The 3-pixel threshold compares the pointer with its own conversion into the source's coordinates, not with the
   grab point (WDD1).
2. Registering a new proxy (slot 13) returns false; already registered returns true.
3. `Drop` does not release `W`'s capture; the next move (no drag) cancels and releases it.
4. Any key on a proxy window cancels a running drag; only Escape does on `W` itself.

Cases:
1. Press on a proxy at (40, 10): a pending drag of type `0xCC0CAE33`. Release without moving: `CancelDrag` (nothing
   sent).
2. Press, then moves: once the threshold trips, `0xAC0FE76D`, the image is captured and `W` follows; moving over a
   drop window sends it `0xAC085F43`, then `0xAC085F47` on each move; release: `0x8C085F48` to it and the source.
3. Escape during the drag: `0x8C0A1FCC`, `D` cleared, `W` removed at its next paint.

## 6. Function inventory (owner `cGZWinDragAndDropManager`, 48)

Slots and handlers: `0x009CC225`, `0x009CC235`, `0x009CC287`, `0x009CC29D`, `0x009CC2A4`, `0x009CC2E1`, `0x009CC30C`,
`0x009CC32F`, `0x009CC334`, `0x009CC350`, `0x009CC38A`, `0x009CC4C8`, `0x009CC5EF`, `0x009CC601`, `0x009CC669`,
`0x009CC905`, `0x009CCB55`, `0x009CCB9F`, `0x009CCCAA`, `0x009CCD10`, `0x009CCD4F`, `0x009CD0ED`. Helpers:
`0x009CC3C4` (remove `W`), `0x009CC417` (target search), `0x009CC6F3` (clear `D`), `0x009CC740` (show `W`),
`0x009CC7F4`, `0x009CC95C` (`Send`), `0x009CC8AA` (save and set the source's buffer flag), `0x009CC999` (capture the
image), `0x009CCB01`, `0x009CCF46`, `0x009CCF82` (proxy list nodes), `0x009CCF9C` constructor, `0x009CD196` deleting
destructor, thunks `0x009CD032`, `0x009CD03A`, `0x009CD04A`, `0x009CD052`, `0x009CD062`, `0x009CD06A`, `0x009CD072`,
`0x009CD07A`, `0x009CD082`, `0x009CD08A`, `0x009CD092`, `0x009CD09A`, `0x009CD0A2`.

Not covered: the `DragData` constructor and destructor (`0x009CC3DD`, `0x009CC53A`, other owners) and the buffer
slots used for the image copy.
