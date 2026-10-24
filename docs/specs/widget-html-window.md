# HTML window: `cGZWinHTML`

Partial owner contract for GOG 1.1.641, supplementing [widget-toolkit.md](widget-toolkit.md).
Requirements below are *[compat]*. The HTML grammar, document lifetime and element contracts are in
[html-document.md](html-document.md); URL/network services are in [web-network.md](web-network.md).
This file specifies the window-side boundary, rather than repeating the parser.

## 1. Identity, inventory and state

The primary object derives directly from `cGZWin`, not `cGZWinGen`. There is an HTML interface and a
separate message-target interface. The latter handles asynchronous completion; it must not replace the
ordinary `cIGZWin` dispatcher. In particular the exported `cGZWinHTML::DoMessage` at `0x008FBFE5`
is a service-message handler, not the primary window slot 3. QI (`0x008FA40C`) distinguishes the
window, HTML and completion-target pointers. Keep reference counts on the complete object while
honoring the adjustment of each incoming interface pointer.

Primary owner overrides (slots are zero-based):

| Slot | Method | Address |
|---|---|---|
| 0, 4, 5 | QueryInterface, Init, Shutdown | `0x008FA40C`, `0x008FC0F6`, `0x008FA329` |
| 55, 88, 107 | SetArea, GZPaint, SetFillColor | `0x008FAC82`, `0x008FB3F5`, `0x008FA62A` |
| 130, 134, 136 | key down, left down, left up | `0x008FB7FE`, `0x008FAAA8`, `0x008FAB08` |
| 138, 139, 143 | mouse move, mouse wheel, command | `0x008FAB8B`, `0x008FA4EE`, `0x008FA9F4` |
| 148 | deleting destructor | `0x008FBDA1` |

The remaining owner surface groups as follows. These are method families, not a proposed new ABI.

| Family | Entry points / evidence |
|---|---|
| Construction and destruction | `0x008FBC38`, `0x008FB6B2` |
| HTML options and gutters | SetFlag, ClearFlag, IsFlagSet, SetFlags (`0x008FA45D..0x008FA4A8`), GetGutters/SetGutters (`0x008FA4BB`, `0x008FB127`) |
| Document and URL installation | SetURL overloads (`0x008FB168`, `0x008FBBF7`), InternalSetDocument (`0x008FBDE4`), SetDocument (`0x008FBF73`), CancelCurrentLoadRequest (`0x008FA9A7`), GetLoadStatus (`0x008FB733`) |
| Document layout and picking | RecalculateAll (`0x008FA559`), PickElement (`0x008FA57D`), SetHTMLPaintArea (`0x008FAC01`), GetDocumentSize (`0x008FB5E5`) |
| Scroll positions and anchors | GetScrollPosition, GetAnchorPosition (`0x008FA65C`, `0x008FA671`), SetScrollPosition, ScrollToAnchor (`0x008FAD0E`, `0x008FADE4`) |
| Scrollbar management | Image getters/setters (`0x008FA69F`, `0x008FA6C9`, `0x008FAEA0`, `0x008FAF21`), RemoveScrollBar, SetScrollBarValues/Sizes, AddScrollBar, TestForPossibleScrollBarAdditionOrRemoval (`0x008FA6F3`, `0x008FA776`, `0x008FA88A`, `0x008FAFA2`, `0x008FB31A`) |
| Service/style configuration | SetWWWSystem (`0x008FA5F3`), SetTextSizeTable (`0x008FB66F`); completion target `0x008FBFE5` |

Persistent state includes the installed document, a document drawing context, a lock protecting document
replacement/picking, a repaint timer, optional cursor and scrollbar references, URL state and a load
status. The document starts absent and the status starts idle. The draw context caches the current
inherited output buffer; paint refreshes that cache before checking the status.

The status values relevant to this boundary are **0 idle**, **1 loading**, **2 ready**. Status at complete
object `+0x464` is instruction-verified by paint at `0x008FB408`, cancellation at `0x008FA9AF`, and
SetDocument at `0x008FBF94` (the last two receive the HTML interface, so their displayed offset is
relative to that interface). Do not use decompiler offsets from one interface as complete-object offsets.
Other constructor defaults and the full secondary vtable remain open in WTH1.

## 2. Lifecycle and document replacement

First Init (`0x008FC0F6`) initializes the base, acquires cursor resources with fallback, installs the WWW
service if not already supplied (first service lookup, then creation fallback), initializes the HTML draw
context, and establishes its paint area. A previously stored nonempty URL can start loading at this
point. Repeated Init succeeds without rebuilding these resources. This does not establish that all
missing-service paths can subsequently paint or load safely.

`InternalSetDocument` (`0x008FBDE4`) creates a **fresh** `cHTMLDocument`, initializes it against this
window and its configured companion interface/base URL, applies the configured text-size table when
present, and parses the supplied text. Parse failure releases the candidate and returns false, keeping
the currently installed document. This is the parser-facing transaction boundary; do not mutate the
installed tree as input is tokenized.

After successful parsing, installation is serialized by the window's document lock. It adjusts the repaint
timer according to whether the new document needs ongoing updates, establishes the new paint/viewport
state, shuts down and releases the old document, installs the candidate in both window and draw-context
state, updates the document URL, and removes existing scrollbars. Future layout can recreate them for
the new document. Allocation exhaustion is not an observed graceful false-return path: the caller assumes
the fresh document exists before invoking it. A replacement may return a checked allocation error as a
*[choice]*, but must not describe that as measured original behavior.

The public `SetDocument` (`0x008FBF73`) returns false if installation fails. On success it changes status
to ready, sends a status-change notification only if the old status differed, invalidates the window,
and returns true. The notification contains the window pointer, new status and old status. This order
is verified at `0x008FBF8B..0x008FBFCF`: observers see the new status during notification.

Shutdown (`0x008FA329`) does nothing beyond success when uninitialized. Otherwise it cancels the
active load through the HTML interface, stops the repaint timer, releases held service/companion,
cursor, document and scrollbar/image references, clears those references, then shuts down the base.
Destruction calls Shutdown and destroys the URL, timer, lock and HTML drawing-context members before
the base destructor (`0x008FB6B2`). Internal replacement explicitly shuts down the old document;
window Shutdown releases its document reference, so these two paths must not be conflated.

## 3. Loading, cancellation and asynchronous input

`SetURL` (`0x008FB168`) rejects an uninitialized window, absent WWW service, or empty full URL. An
unchanged URL can succeed without starting another request when the caller has not requested refresh.
Starting a request sets loading status and notifies the target before invoking the WWW service.
If the request cannot be started, status returns to idle and another notification is sent; the wrapper
can still return true. Thus its boolean is not a download-success result. URL identity/refresh behavior
while a request is already outstanding, and network request flags, remain WTH1.

`CancelCurrentLoadRequest` has an unusual return contract (`0x008FA9A7..0x008FA9F3`):

| Prior state | Effect | Boolean result |
|---|---|---|
| Loading | Set idle first; notify target of loading-to-idle transition | **false** |
| Any other status | No status change or notification | **true** |

This routine's inspected instructions do not call a WWW cancellation method. Its established effect is
local status cancellation. The completion handler ignores completion work unless status is still loading;
do not assume that the underlying network operation was synchronously stopped.

The separate completion-target handler (`0x008FBFE5`) rejects an unrelated service-message type.
For the recognized type it consumes completions even when loading has already ended. Successful
nonempty response data is passed through InternalSetDocument; parse failure leaves the pending status
unchanged and returns handled. Other completion transitions depend on whether a document already
exists. Status notifications and invalidation are window operations after the service result is examined.
The service record layout, response error mapping, request identity and stale-completion behavior need
WTH1 before an asynchronous adapter is implemented; no undocumented request-generation filter is
implied by this spec.

## 4. Scrolling, layout and drawing

The viewport stores document-space origin plus width/height. SetScrollPosition preserves its extent.
For each axis, with requested position `p`, document extent `D` and viewport extent `V`, the result is
**`max(0, min(p, D - V))`**, using signed comparisons. This is verified at
`0x008FAD33..0x008FAD85`. If neither resulting coordinate changes, there is no scrollbar update or
invalidation. On change the viewport edges move together, the corresponding live scrollbars receive
the new positions, and the window is invalidated (`0x008FAD94` onward). A small document therefore
clamps to the origin even though `D-V` is negative. Do not use unsigned subtraction for this contract.

Scrollbar command handling (`0x008FA9F4`) identifies the vertical or horizontal child by its window
instance ID, changes only that viewport axis, preserves that axis's viewport extent, and invalidates.
Unrecognized commands still return true in this handler. Scrollbar creation/removal and exact ranges
are not settled by the clamp rule above.

Paint (`0x008FB3F5`) is synchronous drawing into the inherited output buffer, not screen presentation.
It updates the HTML context's buffer reference first. Unless status is ready it then returns success
without painting the document (`0x008FB3F9..0x008FB40F`). With no document/body, only an optional
background clear is attempted. With a body, it updates the HTML paint area and invalidates cached
document geometry when that area changes. A body-specified color can update the window's fill color.

Background fill, background-image tiling and optional outline each use a matching begin/end buffer
operation; if acquisition fails the corresponding drawing stage is skipped. Layout and body painting
still proceed. A document needing layout receives a fresh HTML drawing context, layout/finalization,
and then scrollbar reconciliation. The context is prepared afresh again before body paint. This prevents
paint from accidentally retaining the traversal state of layout. Background image phase/fixed-background
options, outlines and draw-context metrics require WTH1 and the element/drawing-helper contracts.

Picking and mouse handlers (`0x008FA57D`, `0x008FAAA8`, `0x008FAB08`, `0x008FAB8B`) operate under
the document lock. They translate window input into document space, ask the body for the hit element,
and route press/release to that element. Link release can initiate anchor navigation. Hover chooses a
link cursor only for a link-capable hit and otherwise uses the ordinary cursor. These hooks inherit the
window's upstream capture/filter dispatch; they do not introduce a new native event pump. Exact
mouse-option masks and keyboard/link notification records remain WTH1.

## 5. Conformance vectors and coverage

| Setup/action | Required result |
|---|---|
| Document width 80, viewport width 100, request x=7 | x=0; no unsigned wrap to a huge position. |
| Document width 300, viewport width 100, request x=250 | x=200. |
| Same extents, request x=-5 | x=0. |
| Request current clamped origin | No invalidation or redundant scrollbar position calls. |
| Replace an installed document with text whose parse fails | False; old document remains installed. |
| Cancel while loading, then cancel again | First false with transition notification; second true without one. |
| SetDocument succeeds while already ready | Invalidate, return true, no ready-to-ready status notification. |
| Paint while loading with an old document still present | Success without painting that document. |
| Background buffer acquisition fails | Skip that background stage; continue document layout/paint. |

The 8,139-byte catch-row owner remains **partial**. This file closes important window/document lifetime,
status and scroll contracts, not every method or secondary slot. WTH1 in
[questions/widget-toolkit.md](questions/widget-toolkit.md) tracks the remaining work. It is not safe to
mark the entire owner specified from the parser spec alone.
