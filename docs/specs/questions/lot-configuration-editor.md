# Native lot configuration editor questions

**LE3 — Complete object geometry, fences and numeric edge cases**

Round-2 resolved subsets: §9.2–§9.4 now specify exact duplicate candidates,
signed fence filtering, full-containment cleanup (correcting the old intersection
claim), containment/cleanup unordered tests, texture append/selection order,
record-allocation null dereferences and the type-7 three-element/four-store
discrepancy. These are no longer requests to retrace.
Final-round §9.5–§9.6 resolves both ToggleFence overloads for ordinary finite
conversions, first-foundation query/insertion/removal, pixel conversion spill
order and every bounds-mover unordered comparison. Remaining scope is invalid/
overflow integer conversion, movement/hit-test and rectangle conversions outside
those helpers, plus physical payload allocation/copy effects.

Resolve physical type-7 payload allocation/copy and failure consequences of
`0x00484850`, remaining movement and hit-test conversions (`0x00485040`,
`0x004851B0`, `0x00483DC0`, `0x0047B310`, `0x0047B390`), invalid/overflow
integer conversions and water/terrain creation helpers (`0x0047C8D0`,
`0x0047C930`). Do not assign NaN expectations from finite-only rules.
The exact final residual inventory is spec §18 and its companion; PARK at 3/3.

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

Resolve semantic sort/collation and name-helper fallback dependencies of the
four selectors (`0x004827D0`, `0x004832E0`, `0x00482DC0`, `0x0047CC30`);
their existing control/row/accept/cancel rules need no retrace. Reconcile row
schemas, enumeration/filtering, preselection, empty selection and accepted/
missing-control behavior of retaining-wall `0x00480910` and foundation
`0x00485D60`. Complete SetLotSize `0x00486080`, foundation/outside menus
`0x00486930`/`0x00485B00`, object-menu `0x00487B30` types 3/5 and exact
detail markers, edge table `0x00A8E6C8`, and the open add/select wrappers
listed in the companion. PARK at 3/3; no fourth round.

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

Reconcile static family resources/Init flags; PostInit view/alpha setup;
destructor/reference-array cleanup; other keyboard/mouse/capture state;
hit-test numeric conversion and grid/resize/inside fallback; drawing,
title/name helpers, 3D preparation and refresh. Callback adjustment and
sort/scroll dispatch themselves are resolved (§12.1), while sort semantics
remain LE4. The exact non-C methods/bytes are in spec §18 and the companion.

<!-- from: core-lotconfig-editor4 (2026-10-22) -->
- **LE6 — Portable integration boundaries:** §9 calls record offsets +0x10/+0x14 minima and +0x18/+0x1C maxima, whereas the existing published lot-configuration record dependency uses the opposite named fields. Is that an owner-specific interpretation? [Editor uses its literal published raw-offset convention; shared decoder remains unchanged.] What are the exact constructor defaults for compatibility logical lengths, minimum byte and ignored numeric field, and the identity path splitter/extension convention and non-ASCII lowercase rules? [Unpublished initial fields are represented as empty/zero placeholders with no claim of constructor completeness; non-ASCII path comparison and unsupported path syntax are blocked. Verification prompt wording/options, text formatting for slope/path initialization and native fence placeholder text are only paraphrased in the spec; portable strings are literal descriptions pending exact contracts.] Fully implementing Read requires a typed bridge to the already-published LotConfigManager::fix, not a host-supplied reconciliation outcome; that bridge remains unsupported.
Shared attribution is exactly five current bodies/427 bytes: `0x00478360`,
`0x00604920`, `0x006049BF`, `0x006049C6`, `0x006049CD`; historical
`0x006049D4`/`0x00604A67` are interior fragments. Direct callers prove reach,
not exclusive ownership. The 109-entry/52,646-byte arithmetic stays resolved.
PARK at 3/3; no owner-complete claim or fourth round.
