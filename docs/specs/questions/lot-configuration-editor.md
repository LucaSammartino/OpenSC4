# Native lot configuration editor questions

## LE1 — Compatibility multi-selection maps and first scalar write-back

In [lot-configuration-editor.md](../lot-configuration-editor.md) §4, reconcile
all rows of the three multi-select controls created by `0x00480D10`, their
selected-index-to-model-bit mapping and optional-control behavior. Also trace
the `0x294053DB` scalar write-back. The decompiler loses stack tracking after
the early control queries; native instructions must establish these mappings.
Known: modal acceptance is `0x42B7C356`, scalar writes and checkbox byte rules
are already documented. No complete-owner credit until these and the exact
remaining body inventory are resolved.

## LE2 — Corrected Save As, identity-path success and new-name source

Complete §8 using the real Save As entry `0x00485390` (vtable word
`0x00A8DAE8`), including browser flags and all reference cleanup. Reconcile
`GetIdentityInfoForLotConfigurationExemplarPath` (`0x0047F200`) native success
returns: its pseudocode's final false return and corrupted stack are not proof
of unconditional failure. Verify key/type identity helpers `0x0047B1F0` and
`0x0047BAD0`, directory matching/path-derived IDs, and the new-configuration
name source in `0x00483B20`. Basic Save, Close and accepted Save As restore/
retain ordering are in §8. No file-format or overwrite semantics inferred.

## LE3 — Complete object geometry, fences and numeric edge cases

Complete §9 with exact duplicate offsets (`0x0047BB40`, caller `0x00484E10`),
type-7 payload sizing/initialisation, texture insertion order and all failure
paths of `0x00484850`. Resolve signed fence filtering in `0x00484330`, both
ToggleFence overloads (`0x0047E890`, `0x00484700`), and AddLotFoundation
`0x00484530`. Verify every x87 unordered comparison in movement/bounds/hit
testing, coordinate conversion widths, and rectangle rounding. Do not assign
NaN expectations from the finite-only rules already specified.

## LE4 — Family/texture/foundation dialogs and complete object popup

Reconcile each control ID, row schema/sort key, row-to-ID map, preselection,
empty selection, accept/cancel and missing-control behavior in the building,
prop, flora and texture selectors (`0x004827D0`, `0x004832E0`, `0x00482DC0`,
`0x0047CC30`), plus retaining wall/foundation/name/size/slope/network-path
dialogs. Complete object-menu types 3/5, detail current-marker branches and
the network edge table `0x00A8E6C8`. Pseudocode loses stack tracking in the
large family dialogs; existing menu dispatch does not establish those rules.

## LE5 — Remaining lifecycle, input, rendering and attribution

Reconcile the exact inventory companion: static resources/cursors and Init
flags; PostInit view/alpha setup; destructor/reference-array cleanup; callback
sort/scroll dispatch and argument adjustment; keyboard/mouse/capture state;
hit-test selection ordering/popups; drawing, title/name helpers, 3D preparation
and refresh. Also determine whether the six `0x00604920..0x00604A67` entries
and the `0x00478360` record constructor are true editor-owned behavior or reach/
neighbour attribution. The measured owner set is 111 entries/52,880 attributed
bytes; the brief's 52,646-byte label is not reproduced by the available snapshot.
