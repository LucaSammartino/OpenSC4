# Generated windows: `cGZWinGen`

This supplements [widget-toolkit.md](widget-toolkit.md) for the concrete generated window, using GOG
1.1.641. Requirements below are *[compat]* unless marked otherwise. The inherited child list, parent
references, filter dispatch, capture, private buffers, clipping, and invalidation are specified there;
this object does not introduce a second window tree. The game window's attached procedure is a
separate object: see [ui-windows.md](ui-windows.md) and [modal-dialogs.md](modal-dialogs.md).

## 1. Identity, state and method inventory

The primary `cIGZWin` table is `0x00ADC678`, with 152 entries. QI additionally accepts generator IID
`0x5386D516`, returns the secondary interface at object `+0xD8`, and AddRefs the window
(`0x00998F83`). The secondary pointer is not a second window. Methods reached through it must adjust
back to the complete object. The base reference-count implementation is shared.

Primary overrides, with slot numbers counted from zero:

| Slot | Method | Entry |
|---|---|---|
| 0 | QueryInterface | `0x00998F83` |
| 3, 4, 5 | DoMessage, Init, Shutdown | `0x0099A276`, `0x0099AFA8`, `0x0099A804` |
| 55, 62 | SetArea, screen-coordinate hit test | `0x0099A0AF`, `0x00998FD7` |
| 74, 85, 88 | SetCaption, SetCursor, GZPaint | `0x00998FB0`, `0x00999B56`, `0x009995E7` |
| 121 | window-coordinate hit test | `0x0099955D` |
| 130, 131 | key down, key up | `0x00999832`, `0x00999004` |
| 134, 136, 138 | left down, left up, mouse move | `0x0099A361`, `0x00999039`, `0x0099A3A7` |
| 143, 148, 151 | command, deleting destructor, AsIGZWin | `0x00999896`, `0x0099B485`, `0x009990A5` |

The remaining owner inventory consists of construction/destruction (`0x0099B6A5`, `0x0099AF6E`);
generator flag mutation (`0x0099B5BF`); procedure access/replacement (`0x00999093`, `0x009998C8`);
background bitmap accessors (`0x009990B0..0x00999159`); user rectangle accessors
(`0x00999065`, `0x00999079`); gutters (`0x0099919B..0x009991D7`); user-data overloads
(`0x00999169`, `0x00999180`, `0x0099A5ED`, `0x0099A90B`); color-bar/divider helpers
(`0x00999204`, `0x009999BA`, `0x00999AFE`); title/side-bar helpers
(`0x0099933C`, `0x009993D7`, `0x00999EB9`, `0x0099A96E`); and internal user/outer rectangle,
placement, size-mode, layout and close helpers (`0x0099922F`, `0x009992AE`, `0x00999BDD`,
`0x00999C15`, `0x00999D43`, `0x009993F7`). This inventory is a reading map, not a claim that every
helper's resource or pixel contract has been settled. In particular, secondary method labels in the
export can be misleading: the apparent `SetBackgroundBitmap` at `0x00999131` is an output getter.

Construction (`0x0099B6A5..0x0099B74C`, checked against instructions) first constructs `cGZWin`.
The generator flag word at `+0xDC`, drag mode at `+0xE0`, remembered hover mode at `+0xE4`, minimum
outer dimensions at `+0xE8/+0xEC`, title-bar reference at `+0xF0`, background at `+0xF8`, procedure at
`+0x10C`, user datum at `+0x110`, and saved cursor at `+0x118` start zero/null. Background blit mode
`+0xF4` starts at **2**. Horizontal and vertical gutter bytes `+0x114/+0x115` both start at **4**.
The constructor enables generator masks **`0x10`, `4`, `1`** via the flag setter. The user rectangle
at `+0xFC..+0x108` is computed during initialization/layout; do not promise its contents before then.

## 2. Procedure attachment and dispatch

The attached procedure supports two entry forms: slot 3 takes `(window, message-record)`; slot 4
takes `(window, type, data1, data2, data3)`. A true result from the record form skips the primitive
form. If a callback clears the attached procedure, do not call the primitive form through the old
field. A temporary reference protects the window while ordinary messages run. The apparent Release
of a message in the decompiler is actually cleanup of that temporary window reference, confirmed by
`0x0099A2B2..0x0099A2F5` and `0x0099A30F..0x0099A35A`.

`DoMessage` has three observable cases (`0x0099A276`):

| Message | Processing and return |
|---|---|
| Layout broadcast `0xA2BF8AD8` | Compare data1 with the window ID. A mismatch returns false. A match requests layout and returns true even if no procedure exists. This bypasses normal procedure/base dispatch. |
| Mouse types **7 through 14 inclusive** | Notify the attached procedure, record form then primitive fallback. Regardless of its handled result, continue through `cGZWin::DoMessage` and return that result. |
| All other types | Offer to the procedure. Either form returning true ends processing with true. Otherwise run the base dispatcher and return its result. |

These boundaries and the low-byte handled tests are instruction-verified at `0x0099A27F..0x0099A35D`.
In particular a procedure cannot suppress ordinary mouse processing simply by returning true.

`SetWinProc` (`0x009998C8`) is identity-sensitive: setting the same procedure does nothing and succeeds.
On replacement the old procedure receives detach notification, then loses its owned reference. The
new procedure is stored and AddRef'd, receives attach notification, then a layout-related notification.
For each notification the record form gets first refusal, with primitive fallback only if still attached.
The event values observed here are detach `0xA2BF8AD6`, attach `0xA2BF8AD5`, and
`0xA2BF8AD8`; these are procedure lifecycle messages, not keyboard events. Reentrant replacement
is possible: callers must not assume the original procedure survives a callback.

## 3. Lifecycle and failure boundaries

`Init` (`0x0099AFA8`) succeeds immediately when already initialized. On the first call it initializes
the base window, optionally obtains the default background from the control manager if no background
is installed, creates the title bar unless generator mask `0x1000` suppresses it, places decorations,
and computes the user rectangle. It subscribes to the layout broadcast when a message server exists,
resets the manager's focus selection, and requests focus for this window.

An attached procedure then receives initialization event `0xA2BF8AD0`. If neither callback form handles
it, `Init` returns false. The preceding base initialization, subscription and decoration work have
already happened; failure is not transactional rollback. Successful procedure initialization is followed
by layout. Without a procedure the initialization succeeds. Missing optional control/message services
are tolerated at their lookup sites; this is not a guarantee that every later draw operation tolerates
missing drawing services.

`Shutdown` (`0x0099A804`) succeeds immediately if the base init flag is clear. Otherwise it hides the
window, unsubscribes from layout if the message server exists, detaches/shuts down/releases the title
bar, releases the background, notifies the procedure with `0xA2BF8AD1`, clears the procedure and
the keyed interface map, then shuts down the base window. Destruction invokes this shutdown and
releases the saved cursor before destroying the base. The deleting destructor's bit 0 controls freeing
the allocation (`0x0099B485`). Shutdown is a lifecycle operation; it does not mean that callers may
continue using raw pointers to released child decorations.

## 4. Client area and layout negotiation

Let horizontal/vertical gutters be unsigned bytes `gx`, `gy`; let outer width/height be `W`, `H`.
`calc_user_area` produces a rectangle in window coordinates:

| Edge | Value |
|---|---|
| left | `gx`, or `2*gx + 8` when generator mask `0x10000` enables the side bar |
| right | `W - gx` |
| top | `gy` without a title bar, otherwise `title-bar height + gy` |
| bottom | `max(top, H - gy)` using signed comparisons |

The gutter byte loads, side-bar addition and signed bottom clamp are verified at
`0x00999237..0x009992A4`. Only the vertical interval is explicitly repaired here; a narrow outer
window can produce right less than left. Do not silently apply the bottom rule to the horizontal pair.

The inverse helper expands a supplied user rectangle to an outer rectangle (`0x009992AE`). Layout
negotiation (`0x00999D43`) gives the procedure a proposed user rectangle plus writable minimum and
maximum dimensions. Defaults are zero minimum and signed maximum dimensions. A handled negotiation
clamps user width and height to the supplied bounds, translates to an outer size, and requests resize
if necessary. If resize is requested, this pass returns before sending final placement; the resulting
SetArea path provides the placement callback. Otherwise the procedure receives the final user
rectangle and outer rectangle. Without a procedure `on_layout` returns false and does no negotiation.

`SetArea` (`0x0099A0AF`) first applies the stored minimum outer dimensions. A size change can invoke
the same procedure negotiation before the base area is committed. It then places decorations and
recalculates the stored user rectangle. A move with unchanged size takes the base-area path without
rebuilding decorations. An attached procedure still receives final placement on that path. This
separates changes in origin from changes in available client space. Exact constraints for inverted
procedure-provided bounds and the full inverse-title geometry remain open; see WTG1 in
[the question file](questions/widget-toolkit.md).

## 5. Input, flags and drawing boundary

Left down (`0x0099A361`) calculates a sizing mode and tries capture when that mode is nonzero.
Failed capture cancels the drag mode. Successful capture saves the input point; subsequent movement
uses displacement from that point to move/resize the rectangle. Left up (`0x00999039`) clears a live
mode and releases capture, returning whether a mode was active. Movement during a move drag
invalidates ancestors; resize invalidates the window. The exact edge-selection and cursor resource
map are not specified by this slice.

Generator flag mutation updates the complete flag word first (`0x0099B5BF`). For a title bar that
already exists, individual title-related masks are forwarded to that child. Special side effects are
selected by equality with the requested mask, not by testing every bit in a combined mask; setting a
combined value therefore need not perform the same decoration work as separate calls. Toggling the
title suppression mask creates/destroys the title decoration; toggling the side-bar mask creates/destroys
the side decoration. Replacing a bitmap owns a reference to the replacement before releasing the old
one and is a no-op for identity (`0x009990EC`).

Key down can close the generated window for Escape or Alt-F4 when the generator close option is
enabled (`0x00999832`); key forwarding to the notification target is inhibited by the base flag tested
there. Command handling recognizes close/minimize-style title commands through `kill_self`
(`0x00999896`, `0x009993F7`). Modal completion and ordinary destruction must follow the distinct
manager paths in [modal-dialogs.md](modal-dialogs.md); do not immediately free a window in its own
input callback. Full close-result constants and the resize/move flag map need the follow-up WTG1 audit.

`GZPaint` (`0x009995E7`) is a background/decorations operation into the inherited draw context and
draw area. A generator paint-disable option returns success without callbacks or drawing. Otherwise
the procedure gets a prepaint event and may replace the built-in background. The built-in alternatives
are a solid/raised fill when no bitmap exists, or bitmap blitting/edge drawing according to the stored
mode. After that stage, the procedure gets both subsequent paint notifications even when it handled
prepaint. The drawing hook is synchronous: rectangle pointers designate call-scoped live state.
Exact blit-mode source rectangles, border pixels and color conversion are deliberately not specified
here; they require drawing-helper contracts, rather than assuming the mode number names a platform
blit operation. Paint does not itself present the screen: inherited Plot/Composite/Present own that work.

User-data and decoration helper inventories in §1 remain partial contracts. In particular, overloaded
getter/setter names in the export are not authority for argument ownership. Do not map them to an
engine API until WTG1 is settled.

## 6. Conformance vectors

| Setup/action | Required observation |
|---|---|
| Newly constructed window | Generator masks `0x10`, `4`, `1` enabled; gutters `(4,4)`; background mode 2; no procedure/background/title reference yet. |
| No title/side bar, outer `100×50`, gutters `(4,4)` | User rectangle `(4,4,96,46)`. |
| Same decorations, outer `6×3` | User rectangle `(4,4,2,4)`: horizontal inversion retained, vertical extent zero. |
| A procedure reports handled for mouse type 7 | Base mouse dispatch still runs; overall result comes from the base. |
| A procedure reports handled for an ordinary nonmouse message | No base dispatch; result true. |
| Layout broadcast with a different window ID | False; neither base dispatch nor layout callback. |
| Attach the same procedure twice | No detach/attach notifications and no reference-count churn on the second call. |
| Capture acquisition fails during a resize press | Drag mode becomes zero; the press handler still succeeds. |
| Procedure refuses first Init | False after partial initialization; caller must arrange cleanup, not assume a pristine object. |
| SetArea moves the window without changing its size | No size-negotiation/decorations rebuild; attached procedure still gets final placement. |

## 7. Coverage boundary

This establishes generator lifetime, callback order, dispatch, client geometry, and the drawing/IO
boundary needed by a core implementation. It is **not full owner closure** for the 7,049 byte-weighted
catch-row owner: user-data overloads, decoration creation, all generator flags, exact drawing helper
semantics, title commands and edge hit tests remain WTG1. Retain partial status in coverage accounting.
