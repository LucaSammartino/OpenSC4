# Standard window controls: answered questions

## WC3 — TreeView exact input, drag and pixel contract

In widget-controls sections 6.4–6.5: settle zero-visible-node keyboard behavior,
exact page-navigation arithmetic, right-click selection, modifier ordering,
shared double-click state, drag start/over/drop/cancel notifications and helper
lifetime. Verify selected gradient arithmetic, expander/connector drawing,
clipping, buffer-access failure and image-list resource failures. Anchors:
`009D1031`, `009D135A`, `009D14BF`, `009D2577`, `009D17A8`, `009D0CB1`,
`009D0888`, helper `009CFD89`–`009D14BE`. No game comparison was requested/run.

**ANSWERED 2026-10-24 (re-widget-controls-wc7):** DIFFERS — Sections 6.5–6.7
and vectors T37–T51 settle the owner contracts. Navigation on an empty visible
sequence is an unsafe pre-begin dereference. Page movement uses the ceil row
count without subtracting an overlap row. Control toggles only with flag 0x10;
Shift has no range rule. Static timer/rectangle state permits cross-instance
double clicks. Drag requires a synchronous acceptance byte, borrows its helper
data during the runner call, selects/rejects targets by ancestry, and sends a
drop pair without moving nodes; empty target selection cancels without restoring
the old selection. No separate owner cancel notification is present. The
selected effect is two flat fills with byte-average/quarter-band arithmetic.
Access failure skips its guarded primitive, not the whole paint. Exact final
raster pixels and modal termination remain the called graphics/font/runner
service contracts, with no game-verified bitmap claim.

## WC2 — TreeView property/node ABI and lifecycle residuals

In widget-controls sections 6.1–6.3, settle remaining primary-slot signatures,
descriptors/child output, ordering, move/removal/wrappers, ownership, restart
and resource failures; verify the primary table has 60 entries.

**ANSWERED 2026-10-24 (re-widget-controls-wc7):** DIFFERS — Section 6.6
and vectors T19–T36 settle these contracts. Insert and get/set descriptors
differ. Insertion references insert before; move's positive references insert
after and missing references wrap to the beginning. Move has no cycle guard.
Removing root clears descendants but preserves root. Child/selection arrays
have no capacity argument. Nodes borrow fonts and image indices; the image
vector retains buffers. Image mutation grows metrics without invalidation.
Init has no aggregate resource rollback, Shutdown leaves stale root/init
state, and later Init is not a fresh supported restart. Malformed allocation
paths can dereference null or leak an already-constructed insertion.

## WC1 — ListBox residual adapters, rendering and malformed inputs

In sections 5.1–5.5, settle folded aliases, suppression nesting, clear/removal,
comparisons, style/gutter/bar setters, delivery ABI, malformed removal,
allocation and destruction, color/ellipsis/intensity and bar capture.

**ANSWERED 2026-10-24 (re-widget-controls-wc7):** DIFFERS — Section 5.6
and vectors L27–L36 specify all residual slots. Suppression was a mistaken
interpretation of sorting adapters. Some negative removals decrement count
without removing storage. NaN intensity survives, fractional intensity draws
no fill, short color prefixes erase temporary text and failed scans reuse
uninitialized/prior RGB words. Font fitting receives the original prefix;
the owner appends no ellipsis. Move and left-up use opposite capture tests.
Shutdown and destruction have separate resource domains; allocation failure
has no graceful owner return or rollback contract.

## WC4 — ListBox removal, drop sizing and next page

Filed by core-widgets5 on 2026-10-23: does removing the selected row retain
the index, how does origin adjust, and what are drop sizing and next-page rules?

**ANSWERED 2026-10-24 (re-widget-controls-wc7):** DIFFERS — Section 5.3 now
specifies that a selected row with a successor retains its index; only a
selected last row or removal before selection decrements it. Origin is
decremented only for removal before it, then clamped. Enabled drop height and
width policies run on insertion/replacement/removal and Init. Height clips
to parent bottom minus two pixels and whole lines; width measures raw stored
strings. NextPage requests first+wholeRows−1 then selects requested+wholeRows,
which can move origin again. Vectors L19–L25.

## WC5 — ListBox color escape recognition

Filed by core-widgets5 on 2026-10-23: which leading bytes introduce the color escape?

**ANSWERED 2026-10-24 (re-widget-controls-wc7):** MATCHES — The provisional
plain-text boundary can be extended: section 5.5 specifies ASCII `#` as the
introducer and `##` as a literal escape that removes one byte. A color prefix
uses seven bytes; recognition does not validate its hex suffix. Stored text
is unchanged. Vector L26. Malformed scan results remain in WC1.

## WC6 — TreeView root visibility and geometry

Filed by core-widgets5 on 2026-10-23: which flags include/hide the root,
what are indentation, viewport-count, image-enable and hit-rectangle rules,
and which selection entry anchors navigation?

**ANSWERED 2026-10-24 (re-widget-controls-wc7):** DIFFERS — Sections 6.3 and
6.5 now specify flag `0x2` root inclusion, `0x4` images, `0x8` connectors,
depth/rectangle arithmetic and viewport-derived floor/ceil row counts.
Navigation anchors on the last selected entry, not the first. Hidden-root
children are traversed even when that root is collapsed. Inclusive hit edges
and unsigned image-centering underflow are preserved quirks. Vectors T13–T18.
