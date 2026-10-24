# Standard window controls: open questions

Existing Grid, TextEdit and FileBrowser questions stay with their owner specs.
Target: Windows GOG 1.1.641. Filed 2026-10-23 by re-widgets-wg5.

## WC2 — TreeView property/node ABI and lifecycle residuals

In [widget-controls.md](../widget-controls.md), sections 6.1–6.3: give exact
signatures and behavior for primary slots marked WC2, complete descriptor
output/child enumeration, move/removal and wrapper semantics. Verify insertion
ordering sentinels −2/−3/−4 and explicit-reference behavior through the list
helpers called by `009D3A7A`. Determine invalid-reference post-allocation state,
node/image ownership, restart after Shutdown and resource-failure outcomes.
Primary table `00AE3240` has 60 entries, not the following float/data word.

Partial progress 2026-10-24 (re-widget-controls-wc7): section 6.6 settles O3–18,
O37/40/41, both descriptor layouts and child output, insertion ordering,
invalid-reference allocation/handle leak, borrowed node fonts and stale-root
restart. Remaining WC2 work: O19–24 image/insertion wrappers, O26–28 exact
move/removal/wrapper contracts, O31–36/38/42–44/48–55 node and selection
adapters, image ownership/resource failures. Keep this question open until
those contracts are documented; do not re-file the settled portions.

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

