# Standard window controls: open questions

Existing Grid, TextEdit and FileBrowser questions stay with their owner specs.
Target: Windows GOG 1.1.641. Filed 2026-10-23 by re-widgets-wg5.

## WC1 — ListBox residual adapters, rendering and malformed inputs

In [widget-controls.md](../widget-controls.md), sections 5.1–5.5: settle the
slots marked WC1, including folded count/selection aliases, suppression nesting,
clear-all/removal wrappers, exact prefix comparison, style/gutter/bar setters
and notification-mode setter ABI. Determine negative removal index behavior,
failed allocations and repeated Shutdown/destruction domains. Inspect
`009CA19A` color-prefix/ellipsis behavior, fractional highlight intensity and
`009C8A3F` NaN/limit behavior using instructions. Resolve the asymmetric bar
capture tracking in `009C9623/009C9769`. Do not rely on exported method names:
native W slots 158/159 remove/insert storage, while L slots 10/11 select.

## WC2 — TreeView property/node ABI and lifecycle residuals

In [widget-controls.md](../widget-controls.md), sections 6.1–6.3: give exact
signatures and behavior for primary slots marked WC2, complete descriptor
output/child enumeration, move/removal and wrapper semantics. Verify insertion
ordering sentinels −2/−3/−4 and explicit-reference behavior through the list
helpers called by `009D3A7A`. Determine invalid-reference post-allocation state,
node/image ownership, restart after Shutdown and resource-failure outcomes.
Primary table `00AE3240` has 60 entries, not the following float/data word.

## WC3 — TreeView exact input, drag and pixel contract

In [widget-controls.md](../widget-controls.md), sections 6.4–6.5: settle
zero-visible-node keyboard behavior, exact page-navigation arithmetic,
right-click selection, modifier ordering, shared double-click state, drag
start/over/drop/cancel notifications and helper lifetime. Verify selected
gradient arithmetic, expander/connector drawing, clipping, buffer-access failure
and image-list resource failures. Anchors: `009D1031`, `009D135A`, `009D14BF`,
`009D2577`, `009D17A8`, `009D0CB1`, `009D0888`, helper `009CFD89`–`009D14BE`.
Current drawing section defines interface calls and model dependence, not
pixel-exact rendering. No game comparison has been requested or run.

<!-- from: core-widgets5 (2026-10-23) -->
- **WC4 - ListBox section 5.3 removal and drop sizing:** after removing the
  selected row, does selection move to the previous row or retain its index
  when a following entry exists? What is the exact scroll-origin adjustment?
  Please specify drop-mode sizing on insertion, text replacement and Init,
  plus next-page arithmetic (only previous-page arithmetic is given).
  [Literal provisional removal decrements selection at/after the removed row
  and origin after the removed row; next-page/drop sizing remain open under
  WC1's adapter boundary. No host-control behavior is assumed.]

<!-- from: core-widgets5 (2026-10-23) -->
- **WC5 - ListBox section 5.5 color escape recognition:** which leading byte
  sequence identifies a color escape? WC1 leaves parsing/ellipsis unresolved,
  but even distinguishing ordinary text from the escape branch requires the
  introducer. [Only a plain-text drawing subdomain can be claimed until WC1
  supplies this predicate; no escape byte is inferred.]

<!-- from: core-widgets5 (2026-10-23) -->
- **WC6 - TreeView sections 6.3/6.5 root visibility and geometry:** which flags
  include or hide the root in the flattened sequence, and what are the precise
  indentation, viewport-count, image-enable and hit-rectangle rules? Which
  selection entry anchors keyboard navigation when several nodes are selected?
  [Literal hierarchy flattening includes root and follows child-list order;
  viewport count is explicit input, first selection anchors navigation.
  Pixel/layout and native insertion mapping remain open under WC2/WC3.]
