# Native lot configuration editor questions

LE3/LE4/LE5/LE7 are archived with measured partial answers. LE9–LE18 replace
their unresolved scope; retiring the broad entries does not close the owner.
LE6/LE8 remain answered and are not reopened.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE4 -->
**LE11 — Retaining-wall and foundation selector bodies**

Resolve 0x00480910 and 0x00485D60 row schemas, enumeration/filtering,
preselection, empty selection, acceptance, missing-control and cancel/failure
behavior. Foundation popup dispatch is §26.2; the dialog itself remains open.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE4 -->
**LE12 — SetLotSize arithmetic and refresh**

Resolve 0x00486080 scale selection, rectangle sizing, integer/float widths,
byte truncation, degenerate dimensions and dirty/refresh order. The size
dialog wrapper in §16 is already resolved.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE4 -->
**LE13 — Selected-object popup remainder and add/select wrappers**

Resolve 0x00487B30 types 3/5, exact detail markers, edge table 0x00A8E6C8
and add/select wrappers still marked non-C in the inventory companion.
Separate command effects from dialog implementations. Foundation/outside
popups are §26.2, including their differing confirmation result IDs.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE4/LE7 -->
**LE14 — Shared grid sort/search semantics and selector name fallbacks**

Establish direction/collation for grid +0x20C and flag/range meaning for
+0x214 in selector/chooser calls. Determine whether row zero can match;
raw start arguments are not a semantic exclusion. Resolve name-helper
fallbacks for selectors 0x004827D0, 0x004832E0, 0x00482DC0 and 0x0047CC30.
Callback dispatch, prop letter-search byte handling and raw arguments are
already specified. Malformed hex parsing remains RZAtoUint32HexA.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE5 -->
**LE15 — Static family resources and PostInit view/alpha setup**

Resolve static family path/name-resource parsing and PostInit view, alpha
and model setup, using the inventory companion's exact non-C methods.
Init once timing/section names and LE8 Shutdown field mapping are closed.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE5 -->
**LE16 — Edit input, capture and cycling remainder**

Resolve keyboard/mouse/capture and cycling interiors beyond §11, §17 and
§21 wrapper dispatch. Hit numeric/fallback geometry is LE9; water/terrain
mode commands and both movement overloads are closed in §25.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE5 -->
**LE17 — Rasterization, name/help and 3D preparation**

Resolve ordinary drawing interiors, remaining object-name/help helpers,
3D preparation and refresh. Preserve acquisition/lock/release, reverse
draw order, marker and height-label contracts. SetTitle is closed in §25.3.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE5 -->
**LE18 — Shared constructor/decoder attribution**

Resolve source ownership separately from reachability for the five shared
bodies/427 bytes listed in §18. Editor dependency is established; exclusive
ownership is not. Historical interior fragments are not additional bodies.

