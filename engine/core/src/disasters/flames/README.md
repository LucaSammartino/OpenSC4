# Fire-flames portable body inventory

Settled source: fire-flames.md §§2.1–2.3, 4.1–4.4, 8 and answered FF3/FF4.
Existing public RzRandom, stream and Xform primitives are reused. No analysis-side
files were consulted. This component has no runtime/game binding or C ABI export.

| Published body | Coverage | Portable implementation |
|---|---|---|
| cSC4Flame::Read (0x004700A0) | covered | `Flame::read`; existing InStream and modern/legacy Xform readers |
| cSC4Flame::Update (0x004710D0) | qualified stages 0–4, side zero | stop(0), clear/release, create, temporary transform, start(1); failed creation loses old effect |
| cSC4Flame::UpdateTransform (0x00470CA0) | exact side zero only | temporary counter=2/kind=5, signed zeros, embedded transform unchanged; null effect returns before trig |
| Triangle helper (0x00470EC0) | restricted PC24 domain | distinct area/normal origins and spills, sequential F cumulative weights |
| GetPointsOnModel (0x00471CF0) | restricted PC24 domain and safe physical backing | ignored first-index offset, complete physical triples beyond count, raw clock-seeded RzRandom draws, ordered placement stores |
| Fire occupant::CreateFlames (0x00472380) | covered on empty vector, FF2–FF4 limits | model path or inclusive box path; flora/top/wall placement; persistent box RNG |
| Fire occupant::SetStage (0x004728B0) | qualified visual stage state | initial -1; every equal raw stage quiet; changed stage stored first, invalid table entries Unsupported |
| Fire occupant::Init (0x00471170) | covered per fire-flames §4, FF2 discrepancy | base Init, join group 0x5005, update existing flames |
| Fire occupant::ViewUpdate (0x00472790) | covered | model-only resampling, all-or-nothing point count, transform/visibility only |
| Fresh flame/visibility/visual Shutdown | qualified published paths | `FlameInitial::native` requires typed unknown shared position word; raw visibility-byte comparison, activity query only for made effects |
| Direct destruction | distinct logical storage operation | `into_effect`/`destroy_storage` transfer references without Shutdown/pointee calls; ordinary Rust RAII Drop is not asserted native-equivalent |
| Full fire occupant owner (target, counters, hosing, identity, reference ledger, shutdown, bounds forwarding, persistence) | **outside this visual component** | disasters §5.11 documents several methods; this is not a complete fire-occupant implementation |

No complete-owner credit is claimed. §4.4 reconciles legacy 16/3513 as flame plus
reachability-helper scope; 21/3740 is an inventory, not owner completion. FF2's
twelve helper operand contracts and inherited shared-slot membership remain open;
FF1's world-visual semantic question also remains open.
The SDK's GetHosed/SetHosed/GetFamily/SetFamily labels differ from the behaviour
spec's numeric hosing and fire-ID descriptions; no new ABI interpretation is made.

`FlameInitial::native` supplies published 0/1/0/0 defaults with a required
`SharedPositionWord`; explicit initial states remain available for saved/test
compositions. No new made-byte transition or automatic unhide is invented.
Shutdown's initialized=false path leaves storage alone. Explicit storage transfer
keeps the caller responsible for reference lifetime; default Rust RAII releases
owned references as a safe portable lifetime policy, not a native destructor.
Mesh backing is borrowed and never released.

There is no duplicated RNG, transform codec or geometry library. The private
`numeric` module implements only the owned FF3 operation/store associations.
It assumes explicit PC24, nearest/even, masked exceptions, not all-thread ambient
control. Finite normal binary32/zero inputs and 24-bit registers with exponents
[-400,400] are qualified. Sqrt requires a verified exact dyadic root; reciprocal
uses exact midpoint products. PC53, irrational roots, subnormal/nonfinite input,
degenerate normals and nonfinite/subnormal reloaded stores are Unsupported.
Discarded row-2 float copies may overflow while the register survives. No host
chooses points, normals, areas, transforms or the final placement verdict.
Unbacked reads and one-past selection are Unsupported, never false, clamped,
skipped, renormalized or replaced with a fallback triangle. Nonzero-angle x87
FSIN/FCOS remains Unsupported; no libm approximation is claimed native.
Factories and effect methods are foreign primitives; none returns an owner
algorithm's answer. Getters borrow immutable state and contain no RefCell.

Tests use hand-derived published vectors for effect names, side-zero yaw, box
count/top height, side inequalities, area boundaries, barycentrics and transformed
points; plus effect reference ordering, stream failures, malformed meshes,
zero counts/unsupported zero area, flora, stage idempotence and view lifecycle. No runtime binding,
shadow, replacement, or real-game exactness is claimed.
