# Native lot configuration editor questions

**LE3 — Complete object geometry, fences and numeric edge cases**

Round-2 resolved subsets: §9.2–§9.4 now specify exact duplicate candidates,
signed fence filtering, full-containment cleanup (correcting the old intersection
claim), containment/cleanup unordered tests, texture append/selection order,
record-allocation null dereferences and the type-7 three-element/four-store
discrepancy. These are no longer requests to retrace. Remaining scope below is
Final-round §9.5–§9.6 resolves both ToggleFence overloads for ordinary finite
conversions, first-foundation query/insertion/removal, pixel conversion spill
order and every bounds-mover unordered comparison. Remaining scope is invalid/
overflow integer conversion, movement/hit-test and rectangle conversions outside
those helpers, plus physical payload allocation/copy effects.

Complete §9 with exact duplicate offsets (`0x0047BB40`, caller `0x00484E10`),
type-7 payload sizing/initialisation, texture insertion order and all failure
paths of `0x00484850`. Fence filtering, toggles, foundation and bounds-mover
unordered tests are established and need no retrace. Verify remaining x87
unordered comparisons in movement/hit testing and rectangle rounding. Do not assign
NaN expectations from the finite-only rules already specified.

**LE4 — Family/texture/foundation dialogs and complete object popup**

Round-2 resolved subsets: §12 gives all four selector control IDs, enumeration
sources/flags, displayed row contents, preselection, accepted ID parsing/append,
empty selection, cancel/missing-control cleanup and raw sort calls. Remaining
selector work is the semantic interpretation of sort options/collation and
name-helper fallbacks; the other dialogs/object-menu branches below remain open.

Final-round §16 resolves the lot-name, lot-size UI wrapper, slope and
network-path dialogs, including accepted missing-control results, slope
unordered/validation ordering and payload growth; §16.2 proves the fence-options
popup is a native placeholder. SetLotSize's own arithmetic, retaining-wall and
foundation dialogs and remaining selected-object menu branches are still open.

Reconcile each control ID, row schema/sort key, row-to-ID map, preselection,
empty selection, accept/cancel and missing-control behavior in the building,
prop, flora and texture selectors (`0x004827D0`, `0x004832E0`, `0x00482DC0`,
`0x0047CC30`), plus retaining wall/foundation/name/size/slope/network-path
dialogs. Complete object-menu types 3/5, detail current-marker branches and
the network edge table `0x00A8E6C8`. Pseudocode loses stack tracking in the
large family dialogs; existing menu dispatch does not establish those rules.

**LE5 — Remaining lifecycle, input, rendering and attribution**

Round-2 resolved subsets: §12.1 specifies callback adjustment, sort-control
dispatch and prop-only first-byte letter search. §13 establishes reverse object
draw order, type-3 marker path and height-label zero/unordered/sign behavior.
The record constructor is an editor dependency (§9.4); exclusive ownership is
not proved. Remaining lifecycle/input/hit-test/render/attribution work below
continues to block complete-owner credit.

Final-round §17–§17.1 resolves cursor-resource mapping/replacement, mode 4/8
switches, condition-byte cycling, drawing-style output bytes, and hit-test
selected/forward-vector/popup priority. Hit-test numeric/fallback behavior,
other lifecycle/input/render phases and shared-helper ownership remain open.

Reconcile the exact inventory companion: static resources/cursors and Init
flags; PostInit view/alpha setup; destructor/reference-array cleanup; callback
sort/scroll dispatch and argument adjustment; keyboard/mouse/capture state;
hit-test selection ordering/popups; drawing, title/name helpers, 3D preparation
and refresh. Also determine whether the six `0x00604920..0x00604A67` entries
and the `0x00478360` record constructor are true editor-owned behavior or reach/
neighbour attribution. Inventory arithmetic is resolved in the companion:
111 old attribution rows map to 109 current exported entries/52,646 body bytes.
The remaining question concerns behavior/ownership, not the numeric total.
