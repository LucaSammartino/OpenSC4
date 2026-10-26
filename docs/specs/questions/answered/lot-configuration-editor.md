# Native lot configuration editor answered questions

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE5 -->
**LE15 — Static family resources and PostInit view/alpha setup**

**ANSWERED 2026-10-26 (re-le30):** MATCHES — §33 specifies Sys/FamilyNames.ini path construction, entry radix/overwrite behavior, view/buffer/alpha/model call order and failure/retry behavior. Worked cases and exact callback/PostInit/Init closure mapping are included.

Resolve static family path/name-resource parsing and PostInit view, alpha
and model setup, using the inventory companion's exact non-C methods.
Init once timing/section names and LE8 Shutdown field mapping are closed.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE5 -->
**LE16 — Edit input, capture and cycling remainder**

**ANSWERED 2026-10-26 (re-le30):** DIFFERS — §34 specifies input/capture/drag pipelines and live building/flora cycling, with no lot-ID-vector edit. Absent current ID reads list-end: LE16-R specifies the exact guarded-memory observation; no heap-independent result is promised. Worked cases and row mapping distinguish valid cycling from that malformed state.

Resolve keyboard/mouse/capture and cycling interiors beyond §11, §17 and
§21 wrapper dispatch. Hit numeric/fallback geometry is LE9; water/terrain
mode commands and both movement overloads are closed in §25.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE5 -->
**LE17 — Rasterization, name/help and 3D preparation**

**ANSWERED 2026-10-26 (re-le30):** MATCHES — §35 resolves type names, shared help, redraw/message wrappers and drawing order; LE17-R1–R4 narrowly specify remaining raster/3D/texture observations with fixtures, call arguments and buffer/terrain evidence. Worked cases and row mapping explicitly retain partial drawing/3D/resource rows; no complete-owner or measured runtime claim.

Resolve ordinary drawing interiors, remaining object-name/help helpers,
3D preparation and refresh. Preserve acquisition/lock/release, reverse
draw order, marker and height-label contracts. SetTitle is closed in §25.3.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE5 -->
**LE18 — Shared constructor/decoder attribution**

**ANSWERED 2026-10-26 (re-le30):** MATCHES — §36 retains five shared dependencies/427 current bytes, separates actual entries from historical interiors and prevents exclusive editor credit. LE18-R records concrete shared-path reachability; exclusive original source provenance cannot be proved by runtime absence and would need an unavailable original symbol/link map. Worked accounting case and R-row mapping included.

Resolve source ownership separately from reachability for the five shared
bodies/427 bytes listed in §18. Editor dependency is established; exclusive
ownership is not. Historical interior fragments are not additional bodies.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE4 -->
**LE13 — Selected-object popup remainder and add/select wrappers**

**ANSWERED 2026-10-26 (re-le30):** MATCHES — §31 completes types 3/5, one-or-no detail marker, all 20 edge keys/labels, add-mode clear/accept/cancel behavior and texture wrapper, with worked cases and named closure rows. Dialog implementation is §29, not inferred from dispatch.

Resolve 0x00487B30 types 3/5, exact detail markers, edge table 0x00A8E6C8
and add/select wrappers still marked non-C in the inventory companion.
Separate command effects from dialog implementations. Foundation/outside
popups are §26.2, including their differing confirmation result IDs.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE4/LE7 -->
**LE14 — Shared grid sort/search semantics and selector name fallbacks**

**ANSWERED 2026-10-26 (re-le30):** DIFFERS — §32 resolves primary-grid slot targets, ascending/descending CRT collation, texture comparator, half-open search ranges/counts and exact-match flag; chooser row zero is searchable. Family/individual fallback rules, worked cases and selector/chooser closure mapping are included.

Establish direction/collation for grid +0x20C and flag/range meaning for
+0x214 in selector/chooser calls. Determine whether row zero can match;
raw start arguments are not a semantic exclusion. Resolve name-helper
fallbacks for selectors 0x004827D0, 0x004832E0, 0x00482DC0 and 0x0047CC30.
Callback dispatch, prop letter-search byte handling and raw arguments are
already specified. Malformed hex parsing remains RZAtoUint32HexA.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE4 -->
**LE11 — Retaining-wall and foundation selector bodies**

**ANSWERED 2026-10-26 (re-le30):** MATCHES — §29 specifies unfiltered enumeration, two-column rows, preselection, selection capacities, empty OK writes, return values and lifetime/failure order, with worked wall/foundation cases and both selector closure rows.

Resolve 0x00480910 and 0x00485D60 row schemas, enumeration/filtering,
preselection, empty selection, acceptance, missing-control and cancel/failure
behavior. Foundation popup dispatch is §26.2; the dialog itself remains open.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE4 -->
**LE12 — SetLotSize arithmetic and refresh**

**ANSWERED 2026-10-26 (re-le30):** DIFFERS — §30 specifies full-unsigned comparisons versus low-byte storage, aspect-dependent x87 scale selection, outer sizing using the horizontal margin twice, degenerate values and conditional dirty/unconditional draw order; worked cases identify both affected rows.

Resolve 0x00486080 scale selection, rectangle sizing, integer/float widths,
byte truncation, degenerate dimensions and dirty/refresh order. The size
dialog wrapper in §16 is already resolved.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE3/LE5 -->
**LE9 — Hit-test coordinate conversion and fallback geometry**

**ANSWERED 2026-10-26 (re-le30):** DIFFERS — §27 supplies exact reciprocal spills, integer hit tests, grid-before-resize fallback and unbounded negative-side strips; masked-invalid objects may hit zero. Worked cases and DoHitTest closure mapping are in the section.

Resolve 0x00483DC0 numeric conversion, invalid/overflow inputs, and
inside/grid/resize fallback after the established selected/forward/popup
priority. Include edge boundary examples and masked-x87 invalid behavior;
do not extrapolate finite-only comparison rules.

<!-- from: re-lotcfg-editor-le10 (2026-10-24), residual LE3 -->
**LE10 — Rectangle conversions and invalid integer results**

**ANSWERED 2026-10-26 (re-le30):** DIFFERS — §28 supplies four signed-32 rectangle members and division order; _ftol2 uses signed-64 intermediate/low-32 return, so masked invalid yields zero. Pixel fence consequences and helper closure mapping are included.

Resolve 0x0047B310 and 0x0047B390 coordinate/rectangle widths, conversion
order and invalid/overflow results, including the shared _ftol2 dependency
used by pixel fence paths. §25 closes both movement overloads, payload
request/copy extent and water/terrain modes; §9.5–§9.6 closes finite fence
toggles and bounds-mover comparisons. No deterministic heap-corruption
outcome is promised for the §25.1 overrun.

**LE3 — Complete object geometry, fences and numeric edge cases**

**ANSWERED 2026-10-24 (re-lotcfg-editor-le10):** DIFFERS — §25.1–§25.2 resolves physical type-7 request/copy extent, water/terrain mode commands and both movement overloads; LER01–05. Remaining hit/conversion geometry is LE9–LE10. No deterministic adjacent-heap outcome is asserted.

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

**ANSWERED 2026-10-24 (re-lotcfg-editor-le10):** DIFFERS — §26.2 closes foundation/outside popup rows and dispatch, including foundation confirmation 0x5301814C; LER11–12. Remaining dialog, size, selected-popup and semantic dependencies are LE11–LE14. Partial closure only.

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

**ANSWERED 2026-10-24 (re-lotcfg-editor-le10):** MATCHES — §25.3 completes configuration-name title composition, nonzero dirty suffix and unconditional invalidation; LER06. Static/view, input, raster/name/help/3D and attribution remainders are LE15–LE18. Partial closure only.

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

2026-10-23 window integration addendum (§§19–22): raw Init flag requests,
static-resource once timing/section names, destructor storage/reference effects,
mouse wrapper region/fallback dispatch, draw acquisition/lock/Release boundaries
and trigger ordering are now specified. These are resolved subsets, not an
answer to the entire LE5 entry.

Reconcile remaining static family path/name resources; PostInit view/alpha setup;
other keyboard/mouse/capture state;
hit-test numeric conversion and grid/resize/inside fallback; drawing,
title/name helpers, 3D preparation and refresh. Callback adjustment and
sort/scroll dispatch themselves are resolved (§12.1), while sort semantics
remain LE4. The exact non-C methods/bytes are in spec §18 and the companion.

**LE7 — Chooser service and grid dependencies**

**ANSWERED 2026-10-24 (re-lotcfg-editor-le10):** DIFFERS — §26.1 identifies directory-file IID 0x856C8CF7, query-reference ownership/retries and native array/duplicate leaks. Deletion requests only the exemplar key (0x6534284A,0xA8FBD372,configuration ID) on the retained persistent segment; LER07–10. Grid semantics are LE14; malformed hex remains foreign.

The chooser's owner contracts, methods, constructor, messages, list rows,
selection/key lookup, edit registration, cancelable/forced close and delete
failure ordering are specified in §§19–22 (2026-10-23, re-lotcfg-editor5).
Remaining dependency questions: identify the interfaces enumerated by PostInit
`0x00480460` through IID `0x856C8CF7`, verify their acquisition/retention
contract and purpose; identify the backing-store interface used by deletion
`0x0047ED90` at virtual `+0x6C` and exact database item scope. Establish grid
sort direction/collation and text search flag/range meaning for `+0x20C` and
`+0x214`; raw calls alone must not become an assumption that row zero is
unsearchable. Malformed hexadecimal row parsing belongs to `RZAtoUint32HexA`.
The complete-owner label remains unsupported while these and LE3–LE5 remain;
LE6/LE8 integration answers are in the spec and answered file.

**LE6 — Portable integration boundaries**

**ANSWERED 2026-10-24 (re-lotcfg-editor-le9):** DIFFERS — Preserve the editor's
literal raw-offset roles without changing the shared decoder; constructors
give compatibility lengths 16/4/9, growth 1, water 0 and roads 8, not empty/zero
placeholders. The ignored control reads water, and initial float controls use
truncation. §24 specifies dotted extension splitting, UTF-8/wide-unit lowercase
composition (foreign non-C case tables remain existing RZS2), literal prompts,
signed decimal slope text, lowercase `%08x` path text and the fence placeholder.
Read must run the published manager Fix on the installed mutable configuration;
the typed bridge is missing engine integration, not missing editor policy.
See [§24 and vectors LEI06–15](../../lot-configuration-editor.md). Original
scope: cross-spec bounds naming, constructor fields, identity path/case rules,
literal text/formatting, Fix bridge and shared attribution. The five current
shared bodies/427 bytes (`0x00478360`, `0x00604920`, `0x006049BF`, `0x006049C6`,
`0x006049CD`) remain foreign; `0x006049D4`/`0x00604A67` are interior fragments;
109 current bodies/52,646 bytes is unchanged. No complete-owner claim.

**LE8 — Window addendum text and integration boundaries**

**ANSWERED 2026-10-24 (re-lotcfg-editor-le9):** DIFFERS — The exact error is
`The exemplar you are trying to delete is either non-existant or write-protected.\n\n`
(two trailing LF bytes), title `No can do`. Shutdown's font is `+0x130`;
§7.1 now specifies every release, clearing order and the unchanged array
elements. Seven cursor references survive for the destructor. Staged chooser
Open/Close may consume the corresponding core editor results, retaining all
core unsupported boundaries; no foreign policy verdict is authorized. See
[§7.1, §20.5 and §23](../../lot-configuration-editor.md), vectors LEI01–05.
Original scope: full file-unavailable text, exact Shutdown field mapping/order,
cursor retention, and staged core/window Open/Close integration.

**LE1 — Compatibility multi-selection maps and first scalar write-back**

**ANSWERED 2026-10-22 (re-loteditor-window4):** DIFFERS — The three grids use
row+1 as the model bit, clear the entire vector before rebuilding, and require
all three controls on acceptance despite guarded initialisation. Control
`0x294053DB` has no write-back at all. All 15/3/8 descriptions, bit bases,
selection-record widths, ordering and missing-control effects are specified in
[§4.1](../../lot-configuration-editor.md), from native instruction operands.

Original scope: reconcile all rows and selected-index maps in `0x00480D10`,
optional controls, and the first scalar write-back lost by the decompiler.

**LE2 — Corrected Save As, identity-path success and new-name source**

**ANSWERED 2026-10-22 (re-loteditor-window4):** DIFFERS — Native identity-path
resolution returns the saved success byte, not the decompiler's unconditional
false. Full lowercased directory equality precedes provider stem/extension key
derivation; success installs the original path, derived key/ID and retained
references. Key lookup ignores the path-query return; type lookup uses final
group `0xA8FBD372`. New configurations are named `New Lot`. Save As sets window
flag `0x100`, tears down the browser before persistence, and explicitly transfers
the shallow-copied identity references, releasing old references on ordinary
success or selected references on restore. See
[§8.1, §8.2 and §8.4](../../lot-configuration-editor.md). No downstream file-format
or overwrite semantics inferred.

Original scope: complete real Save As `0x00485390`, browser flags/reference
cleanup, identity-path success `0x0047F200`, key/type helpers
`0x0047B1F0`/`0x0047BAD0`, directory matching/path-derived IDs and the new-name
source in `0x00483B20`.
