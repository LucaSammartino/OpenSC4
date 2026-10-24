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

