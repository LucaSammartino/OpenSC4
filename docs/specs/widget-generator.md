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
`0x00999C15`, `0x00999D43`, `0x009993F7`). Secondary method labels in the export can be misleading:
the apparent `SetBackgroundBitmap` at `0x00999131` is an output getter. See §8 for verified contracts.

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
separates changes in origin from changes in available client space. The bound precedence and the
asymmetric inverse-title geometry are specified in §8.3.

## 5. Input, flags and drawing boundary

Left down (`0x0099A361`) calculates a sizing mode and tries capture when that mode is nonzero.
Failed capture cancels the drag mode. Successful capture saves the input point; subsequent movement
uses displacement from that point to move/resize the rectangle. Left up (`0x00999039`) clears a live
mode and releases capture, returning whether a mode was active. Movement during a move drag
invalidates ancestors; resize invalidates the window. See §8.4 for edge and cursor selection.

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
input callback. See §8.4 for the command constants and move/resize flag map.

`GZPaint` (`0x009995E7`) is a background/decorations operation into the inherited draw context and
draw area. A generator paint-disable option returns success without callbacks or drawing. Otherwise
the procedure gets a prepaint event and may replace the built-in background. The built-in alternatives
are a solid/raised fill when no bitmap exists, or bitmap blitting/edge drawing according to the stored
mode. After that stage, the procedure gets both subsequent paint notifications even when it handled
prepaint. The drawing hook is synchronous: rectangle pointers designate call-scoped live state.
The draw-context calls and source rectangles are specified in §8.5; context sampling and color
conversion follow [gz-graphics.md](gz-graphics.md). Paint does not itself present the screen:
inherited Plot/Composite/Present own that work.

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

Sections 1–6 and 8 establish the generator-owned contract, including the secondary interface,
decorations, callback and draw-context boundary. The separately attributed title-bar owner still
requires its own skin/button/font specification; generator calls to that child are specified here.
The low-level raised-rectangle and edge-bitmap helper pixel rules remain the narrow WTG1 remainder;
keep the owner partial until those helpers have an explicit linked contract.

## 8. Secondary interface and remaining generator behavior *[compat]*

### 8.1 Verified secondary table

The generator interface table starts at `0x00ADC5F8`, with 32 slots. This is a single
`+0xD8` view of the complete window. The table words, constructor stores and actual instruction
behavior take precedence over inferred Mac method names.

| Slots | Entries | Contract |
|---|---|---|
| 0–3 | `0079C580`, `007BE710`, `007BE720`, `0095BA6F` | QI/AddRef/Release adjust to the complete object; AsIGZWin returns it without AddRef. |
| 4–6 | `0099908D`, `00999055`, `0099B5BF` | Class ID `0x4386D516`; any-bit flag test; mask mutation with the side effects below. |
| 7–9 | `009998C8`, `009DB800`, `00999093` | Replace procedure; borrow procedure pointer; QI on procedure, false when absent. |
| 10–11 | `00999065`, `00999079` | Copy stored user rectangle out/in, true; setter performs no layout or invalidation. |
| 12–15 | `009990EC`, `009990B0`, `00999131`, `0099911A` | Set bitmap; set `(mode,bitmap)`; get owned bitmap; get borrowed `(mode,bitmap)`. |
| 16–17 | `00999159`, `0099914B` | Set/get integer background mode. No range validation or redraw request. |
| 18–24 | `0099A90B`, `00945EAC`, `00999169`, `0099A5ED`, `00999165`, `00999180`, `0099A643` | Keyed interface set; raw datum set; indexed datum set; keyed interface QI; raw datum get; indexed datum get; keyed removal. |
| 25–28 | `009991C7`, `0099919B`, `009991D7`, `009991AE` | Four-input gutter setter; two-input gutter setter; four-output getter; two-output getter. |
| 29–31 | `009999BA`, `00999AFE`, `00999204` | Add color bar; add horizontal divider; remove child by ID. |

All table entries above are hexadecimal addresses. QI's generator IID differs from its class ID.
The borrowed procedure getter is a folded four-byte accessor, not a zoom operation. Set/get datum
at `0x00945EAC`/`0x00999165` are also mislabeled by the export; they write/read secondary `+0x38`
(complete `+0x110`). The setter succeeds without taking a reference. These are raw 32-bit data,
not interface pointers. Indexed access accepts **only index zero**; other indices return false and
the getter leaves its output untouched (`0x00999169..0x0099919A`).

Keyed data are owned interface references in the map at complete `+0x11C`. A new reference is
protected before the old entry is removed; null removes the entry. Lookup takes `(key,IID,out)`
and delegates QI on the stored interface; missing key returns false without writing output.
Removal of a missing key is a no-op. Clearing the map at Shutdown releases its references.
The ownership helper at `0x004F20C0` is an AddRef wrapper despite its inferred container name;
its instructions and `0x0099A90B..0x0099A96B` confirm replacement lifetime. Keyed set/remove
have no separate reliable boolean success contract; they are mutation operations.

Bitmap setters acquire the replacement before releasing the old reference. The pair setter compares
both mode and pointer, so changing mode with the same pointer can still cause an AddRef/Release pair.
The owned getter tolerates a null output and returns true. With a nonnull output it writes the
pointer then unconditionally calls AddRef: **an absent bitmap is not checked**. The borrowed pair
getter writes both outputs and performs no AddRef (`0x0099911A..0x00999148`). These operations
do not themselves repaint. Callers must supply valid output locations except for the checked owned getter.

Gutters are two unsigned **bytes**, not four independent edge widths. The four-input setter ignores
its third/fourth arguments and forwards the first two; conversion keeps the low eight bits. The
four-output getter returns `(gx,gy,gx,gy)` as zero-extended words; the two-output getter returns
`(gx,gy)`. Neither setter places controls, recalculates the user rectangle or invalidates immediately.
The byte stores and the four-input callee's 16-byte stack cleanup are checked at
`0x0099919B..0x009991D6`; the apparent getter/setter names must not reverse these effects.

### 8.2 Decorations and flags

Flag testing is any-bit intersection, not an all-bits test (`0x00999055`). Mutation succeeds and
updates the word even when optional decoration creation fails. Special work is selected by equality
with the supplied mask; combined masks only update bits unless they equal a special case.

| Generator mask | Meaning / side effect |
|---|---|
| `1`, `2`, `4` | Allow move, allow border resize, enable Escape/Alt-F4 close. |
| `0x10`, `0x20`, `0x40` | Forward to existing title child masks `1`, `4`, `2` respectively. |
| `0x100`, `0x200`, `0x400` | Forward to existing title child masks `0x100`, `0x400`, `0x200` respectively. |
| `0x1000` | Set: destroy title; clear: create title. Recompute user area, then request layout. |
| `0x10000` | Set: create side bar; clear: destroy side bar. Recompute user area, then request layout. |
| `0x2000`, `0x4000`, `0x8000` | Drawing selection only, as in §8.5. |

Title forwarding is instruction-checked at `0x0099B5E7..0x0099B62E`; special cases and layout
at `0x0099B633..0x0099B698`. Unlisted bits remain stored; no additional behavior is asserted.

Title creation is identity-sensitive: an existing title is retained. A new child has ID
`0x42B7C351`, starts at the gutters, clears base mask `0x800`, receives the six forwarded options
above, title flags `0xF` and style value `5`, and the parent's caption. Theme values come from
control-manager keys `0xBB5B2EC6` and `0xBB5B2EC7` (zero fallback when service absent);
the second is reduced to 16 bits. The child is added to the normal window tree, then laid out in
the gutter-inset outer rectangle. Its minimum dimensions plus twice the gutters become the
generator's minimum outer dimensions; an undersized generator requests **both** minimum dimensions.
Child-add failure releases the title and clears the field, returning false. Allocation exhaustion
has no checked recovery before the title's first virtual call (`0x0099A96E..0x0099AC4A`).

Destroying a title removes/releases it, clears the field, and restores minimum dimensions to
`(2*gx,2*gy)`. If a side bar exists it is moved to `(gx,gy,gx+8,H-gy)`. If no title exists,
destruction is a successful no-op (`0x0099933C`). Setting the parent's caption updates the base
caption, then the title caption when present (`0x00998FB0`).

The side bar is a color-bar child with ID `0x42B7C352`, options `(4,13)` and rectangle
`(gx,title.bottom-1,gx+8,H-gy)`; without a title its top is `gy`. Existing ID means no duplicate.
The helper returns false if adding the color bar reports false. It sends the side bar to the back
under temporary base mask `0x800`; an existing title is sent back after it under the same mask.
Creating a title later brings the side bar forward and repositions it below the title, with that
temporary mask (`0x00999EB9`, `0x0099ABEE..0x0099AC4A`). Destroy-side deletes by ID;
an absent child is a successful no-op (`0x009993D7`).

AddColorBar takes `(ID,byte-option1,byte-option2,rect)`. Duplicate child ID returns false.
Otherwise it requests class `0xC2AFA76E`, interface `0xC2AFA76F`, obtains its window, assigns
the ID and area, clears base mask `0x800`, applies the two theme values above, and the two byte
options, then adds the child. Failed ChildAdd asks the window manager to destroy the candidate,
releases the temporary interface and returns false. **Failed class creation returns true without
adding a child** (`0x009999BA..0x00999AFB`). This explains why side-bar creation also checks
whether the child actually exists before repositioning it.

AddDividerH uses options `(3,14)`, top `y`, bottom `y+8`, right `W-gx`, and left `gx-1`
without the side flag or `gx+7` with it (`0x00999AFE..0x00999B4D`). RemoveColorBar deletes a
direct child of the requested ID regardless of its concrete class and returns whether it found one.
Decoration placement enumerates children exposing `0xC2AFA76F`, passing their parent-inset
rectangle `(gx,gy,W-gx,H-gy)` to secondary slot 17. It does not individually resize arbitrary
content children (`0x00999B84..0x00999BDC`, `0x00999BDD`).

### 8.3 Geometry negotiation and picking

The outer expansion helper copies all four user edges and changes them as follows:

| Edge | Expanded value |
|---|---|
| left | `user.left-gx`, or `user.left-2*gx-8` with side flag |
| right | `user.right+gx` |
| top | `user.top-gy` without title; `user.top-title.height-2*gy` with title |
| bottom | `user.bottom+gy` |

This deliberately retains the extra top gutter with a title: it is **not** the mathematical
inverse of §4's client calculation. The height call and doubled gutter subtraction are verified
at `0x00999301..0x00999317`. No edge normalization occurs.

Negotiation event `0xA2BF8AD2` carries data1 pointing to four dimension bounds
`(minWidth,minHeight,maxWidth,maxHeight)`, data2 to the proposed client rectangle and zero data3.
Defaults are `(0,0,0x7FFFFFFF,0x7FFFFFFF)`. Only a handled negotiation applies the proposed
rectangle and clamps dimensions. For each dimension, the minimum test has precedence: below
minimum becomes minimum; otherwise above maximum becomes maximum. Inverted bounds are not
sorted. Final placement `0xA2BF8AD3` passes the stored client rectangle and current outer area.
The stores and signed branch order are at `0x00999D43..0x00999E57` and
`0x0099A0AF..0x0099A20E`. A size-changing SetArea first applies stored outer minima but does
not apply them again after the procedure's result. An unchanged-size move skips decoration/client
recalculation; a previous direct SetUserArea therefore survives that path.

Window-coordinate picking sends `0xA2BF8AD4` to the procedure with data1 equal to
`(low16(x)<<16)|low16(y)`, data2 a pointer to a false-initialized one-byte result, and zero data3.
If either callback handles it, that byte controls the result; merely returning true does **not**
mean inside. Otherwise use base picking. Screen-coordinate picking converts to window coordinates
then calls this virtual picker (`0x00998FD7`, `0x00999571..0x009995D4`).

### 8.4 Dragging, cursors and close commands

Sizing-mode calculation first rejects points inclusively inside the stored client rectangle.
Outside it, resize mask `2` enables gutter edge selection. Left is `x<gx`, right is `x>W-gx`,
top is `y<gy`, bottom is `y>H-gy`; equality belongs to the central interval. Modes are
2 left, 3 right, 4 top, 5 bottom, 6 upper-left, 7 upper-right, 8 lower-left, 9 lower-right.
Move mask `1` overrides the selected border mode with mode 1 if there is no title or the point
lies inclusively within the title's area. No title therefore makes **any nonclient point** a move
point when movement is enabled (`0x00999C26..0x00999D33`).

Dragging uses integer displacement from the remembered local point. Mode 1 translates all edges;
other modes move only their selected edges. Right-edge motion updates remembered x, bottom-edge
motion updates remembered y, while left/top motion leaves those coordinates fixed. Commit goes
directly through **base** SetArea, bypassing generator negotiation/minimum enforcement; moving
invalidates ancestors, resizing invalidates the window (`0x0099A459..0x0099A522`). This path
does not immediately rebuild title/client layout through the generator override.

Without an active drag, cursor selection runs only when **both** move and resize bits are set
and the calculated mode differs from the remembered mode. Missing cursor manager leaves that
remembered mode unchanged. Otherwise it is updated before resource lookup. Modes 2/3 request
cursor 10, 4/5 request 8, 6/9 request 9, 7/8 request 7, and mode 1 requests cursor 1. Missing
resize cursor falls back to 17; missing move cursor has no further fallback. Mode 0 restores
the saved cursor when nonnull, otherwise requests cursor 1. Failure to obtain a resource leaves
the active cursor alone. Successful installation uses base SetCursor with ownership enabled.
Explicit generator SetCursor always replaces its saved reference but only updates the active
cursor when the remembered mode is zero (`0x00999B56`, `0x0099A3A7..0x0099A457`).

Close-enabled Escape (`27`) or Alt-F4 (key `0x73`, modifier bit `4`) invokes close result
`0x42B7C353` and returns true. Otherwise key down/up forward type 5/6, key, modifiers and
zero data3 to the notification target unless **base** flag `0x1000` blocks forwarding; blocked
returns false. Title command type `0x287259F6` accepts only command IDs `0x42B7C353` and
`0x42B7C356`, passing that ID to close; other commands return false (`0x00999832..0x009998C5`).
Close checks the same base flag: modal sends manager command 15 with this window and the
result; nonmodal posts a **type-1 message** to its parent through primary slot 146, with this
window in data1 and zero data2/data3. Base type-1 dispatch eventually calls ChildDelete; parent
message filters/overrides can intercept it. It returns true and does not directly delete storage
(`0x009993F7..0x00999448`, `0x0099CA60..0x0099CA8E`). The nonmodal result ID is not posted.

### 8.5 Drawing interface contract

Generator `0x8000` suppresses all painting including procedure callbacks and returns true.
Otherwise prepaint is `0xA2BF8ACD`; a handled callback skips the built-in background only.
Both later events `0xA2BF8ACE` and `0xA2BF8ACF` still run in that order. Each receives a
pointer to the inherited draw rectangle in data1, zero data2/data3 and the ordinary record-first
fallback protocol. Call-scoped rectangle pointers must not be retained.

Without a bitmap, `0x4000` selects flat fill with inherited fill color when `0x2000` is clear;
with both bits set it draws nothing. Without `0x4000`, clear `0x2000` selects raised fill;
set `0x2000` instead selects only a white/black 3D rectangle outline. With a bitmap,
`0x2000` suppresses the bitmap operation; otherwise mode 0 tiles its full rectangle into the
draw rectangle with offsets `(0,0)`, mode 1 scales its full rectangle through context Blt,
and mode 2 calls the edge-bitmap helper with top-left source cell
`(0,0,floor(bitmap.width/3),floor(bitmap.height/3))`, destination draw rectangle and option 1.
Both divisions are **unsigned integer** divisions (`0x009996DD..0x0099971A`). Other modes
produce no bitmap call. A bitmap path then draws the white/black outline unless `0x4000`
suppresses it; this applies even when `0x2000` suppressed the bitmap itself. See
[gz-graphics.md §3.3](gz-graphics.md#33-blit-binding-and-suspension) for context Blt/BltTiled;
the helper calls' internal edge/raised pixel rules are the remaining WTG1 limitation.
Draw-context failure results are not propagated; GZPaint returns true. A usable context and
bitmap dimensions are caller/resource preconditions, not checked recoverable errors here.

### 8.6 Additional conformance vectors

| Setup/action | Required observation |
|---|---|
| Four-input gutter set `(260,258,9,10)`, then four-output get | `(4,2,4,2)`; no automatic layout. |
| Raw datum set to `0xDEADBEEF`; indexed get at 0 and 1 | Index 0 succeeds with that word; index 1 fails without changing output. |
| Replace keyed interface with itself | New reference protected across removal; no premature destruction. |
| Get missing keyed interface | False; output left as supplied. |
| Owned bitmap get with null output | True without AddRef; nonnull output requires an installed bitmap. |
| No title, client `(4,4,96,46)`, gutters `(4,4)`, inverse expansion | `(0,0,100,50)`. |
| Title height 12, client top 16, gutters `(4,4)`, inverse expansion | Outer top `-4`, retaining the extra top gutter. |
| Negotiation min width 30, max width 20, proposed width 10 | Width 30; no sorting of bounds. |
| Same bounds, proposed width 25 | Width 30, because minimum wins. |
| Procedure handles picking without writing result byte | False; no base fallback. |
| Pick coordinates `(65537,-1)` | Procedure data1 `0x0001FFFF`. |
| Nonclient point, move enabled, no title | Mode 1 overrides a border resize mode. |
| Active right-edge drag by 3 pixels | Right edge and remembered x advance by 3; generator layout is bypassed. |
| Duplicate color-bar ID | False; no factory request. |
| Color-bar factory refuses creation | True; no child added. |
| Combined mask `0x11000` enabled | Both bits stored; neither equality-selected decoration operation runs. |
| Nonmodal close ID `0x42B7C356` | Post to parent: type 1, data1 this window, data2/data3 zero; result ID discarded. |
| Bitmap absent, `0x6000` set | No built-in fill or outline; later paint callbacks still run. |
| Bitmap present, `0x2000` set and `0x4000` clear | Bitmap skipped; outline drawn. |
| Bitmap size `10×8`, edge mode | Source cell `(0,0,3,2)` passed to edge helper. |
| Unsupported bitmap mode, outline enabled | No bitmap call; outline still drawn. |
