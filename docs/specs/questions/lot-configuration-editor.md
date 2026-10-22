# Native lot configuration editor questions

**LE3 — Complete object geometry, fences and numeric edge cases**

Complete §9 with exact duplicate offsets (`0x0047BB40`, caller `0x00484E10`),
type-7 payload sizing/initialisation, texture insertion order and all failure
paths of `0x00484850`. Resolve signed fence filtering in `0x00484330`, both
ToggleFence overloads (`0x0047E890`, `0x00484700`), and AddLotFoundation
`0x00484530`. Verify every x87 unordered comparison in movement/bounds/hit
testing, coordinate conversion widths, and rectangle rounding. Do not assign
NaN expectations from the finite-only rules already specified.

**LE4 — Family/texture/foundation dialogs and complete object popup**

Reconcile each control ID, row schema/sort key, row-to-ID map, preselection,
empty selection, accept/cancel and missing-control behavior in the building,
prop, flora and texture selectors (`0x004827D0`, `0x004832E0`, `0x00482DC0`,
`0x0047CC30`), plus retaining wall/foundation/name/size/slope/network-path
dialogs. Complete object-menu types 3/5, detail current-marker branches and
the network edge table `0x00A8E6C8`. Pseudocode loses stack tracking in the
large family dialogs; existing menu dispatch does not establish those rules.

**LE5 — Remaining lifecycle, input, rendering and attribution**

Reconcile the exact inventory companion: static resources/cursors and Init
flags; PostInit view/alpha setup; destructor/reference-array cleanup; callback
sort/scroll dispatch and argument adjustment; keyboard/mouse/capture state;
hit-test selection ordering/popups; drawing, title/name helpers, 3D preparation
and refresh. Also determine whether the six `0x00604920..0x00604A67` entries
and the `0x00478360` record constructor are true editor-owned behavior or reach/
neighbour attribution. Inventory arithmetic is resolved in the companion:
111 old attribution rows map to 109 current exported entries/52,646 body bytes.
The remaining question concerns behavior/ownership, not the numeric total.
