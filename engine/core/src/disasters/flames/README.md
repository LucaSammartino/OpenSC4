# Fire-flames portable body inventory

Sources: fire-flames.md §§1–4, 8; geometry-utilities.md §§1–2;
model-instances.md §§1–2; foundations.md §6.1; disasters.md §5.11;
public SDK cISC4FireOccupant.h. No analysis-side files were consulted.

| Published body | Coverage | Portable implementation |
|---|---|---|
| cSC4Flame::Read (0x004700A0) | covered | `Flame::read`; existing InStream and modern/legacy Xform readers |
| cSC4Flame::Update (0x004710D0) | covered for published stages 0–4 | owned effect, stop/release/create/apply/start ordering |
| cSC4Flame::UpdateTransform (0x00470CA0) | covered, FF4 precision/sign limits | radian yaw and translation, effect transform applied |
| Triangle helper (0x00470EC0) | covered, FF3 numeric limits | local twice-area, cumulative weights, unit normal, exact wall/side comparisons |
| GetPointsOnModel (0x00471CF0) | covered on published readable mesh domain | first primitive, sequential triples, first cumulative >= draw, biased barycentrics, outward offset, instance transform |
| Fire occupant::CreateFlames (0x00472380) | covered on empty vector, FF2–FF4 limits | model path or inclusive box path; flora/top/wall placement; persistent box RNG |
| Fire occupant::SetStage (0x004728B0) | covered for published stages 0–4 | changed-stage store, create-if-empty, update all; equal stage is a no-op |
| Fire occupant::Init (0x00471170) | covered per fire-flames §4, FF2 discrepancy | base Init, join group 0x5005, update existing flames |
| Fire occupant::ViewUpdate (0x00472790) | covered | model-only resampling, all-or-nothing point count, transform/visibility only |
| Native flame construction/destruction/made/visibility methods, writer, other bodies in the brief's 16-function count | **missing: unpublished** | FF2; explicit `FlameInitial` inputs, no guessed native writer or lifecycle algorithm |
| Full fire occupant owner (target, counters, hosing, identity, reference ledger, shutdown, bounds forwarding, persistence) | **outside this visual component** | disasters §5.11 documents several methods; this is not a complete fire-occupant implementation |

No complete-owner credit is claimed for either native owner. The public spec
names nine visual/helper bodies, not the full sixteen-body inventory counted in
the brief. FF2 requests that reconciliation and the missing lifecycle contracts.
The SDK's GetHosed/SetHosed/GetFamily/SetFamily labels differ from the behaviour
spec's numeric hosing and fire-ID descriptions; no new ABI interpretation is made.

`FlameInitial` supplies unpublished construction flags explicitly. `Stage` only
represents the five published effect-table entries. `PlacementError` is a portable
diagnostic for unpublished malformed data/overflow/reentry/initial-stage cases,
not a claimed native return value. FF3/FF4 document literal numeric and ordering
choices. No native destruction-stop, made-byte transition or automatic unhide is
invented. Rust owns creation references and releases them on replacement/drop;
mesh data is borrowed, never released by the core.

There is no duplicated RNG, transform codec, file format, or vector library:
this module reuses `rng::RzRandom`, `network_occupants::Xform`, `stream::InStream`,
and the existing camera vector operations (made crate-visible only).
Factories and effect methods are foreign primitives; none returns an owner
algorithm's answer. Getters borrow immutable state and contain no RefCell.

Tests use hand-derived published vectors for effect names, radian yaw, box
count/top height, side inequalities, area boundaries, barycentrics and transformed
points; plus effect reference ordering, stream failures, malformed meshes,
zero counts/area, flora, stage idempotence and view lifecycle. No runtime binding,
shadow, replacement, or real-game exactness is claimed.
