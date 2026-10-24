# model-instances: answered questions

<!-- from: core-model-instances4 (2026-10-22) -->
~~- **MI3 - §§1–2, constructor/records:** what are the base/static initial owner, custom data, properties and draw attributes; which bytes in the 20-byte draw-attribute record hold flags; and what is the transform change counter's width and SetTransform increment source? [Rust accepts explicit base defaults; static custom data/properties start zero, supplied draw attributes; semantic flags plus 16 opaque bytes without native packing; wrapping u32 instance-local counter, never an ABI layout]~~

**ANSWERED 2026-10-22 (re-model-instance-contracts4):** DIFFERS — §§1.1–1.2 and 2: both properties default to 2; draw flags occupy byte 0, bytes 2–3 are unwritten; transform counters are bytes, Get increments destination and Set increments instance, ignoring source counter.

<!-- from: core-model-instances4 (2026-10-22) -->
~~- **MI10 - §1 reference lifecycle:** what is the initial native reference count and the behavior when its 16-bit counter overflows? [a returned Rust Reference owns one count; exposed arithmetic wraps at 16 bits, while backing Arc storage remains alive until all Rust handles are released, avoiding invalid memory access on counter overflow; ordinary non-overflow releases destroy the most-derived owner at zero]~~

**ANSWERED 2026-10-22 (re-model-instance-contracts4):** DIFFERS — §§1.1 and 2: constructors start at zero; basic has wrapping 16-bit count, static has wrapping 32-bit count; Release destroys at either zero or one, including after overflow. Safe surviving backing storage is a portable choice, not native lifetime.

<!-- from: core-model-instances4 (2026-10-22) -->
~~- **MI6 - §§2,4 material/processor lifetime:** are SetModel's first-material mutations confined to the instance copy; does processor SetModel return true when a non-format-2 mesh leaves it with only the instance; and do explicit ReleaseModel/destruction restore original vertices? [instance copy; true for a retained instance, false only for explicitly refused multi-frame animation; restore only at the documented first step of SetModel, ReleaseModel/destruction release all references exactly once]~~

**ANSWERED 2026-10-22 (re-model-instance-contracts4):** DIFFERS — §4.1: material mutation is instance-local, but processor SetModel also fails and clears both references for missing, empty or ineligible containers. Destruction restores originals; ReleaseModel alone leaves replacements and a borrowed container pointer, and is not idempotent.

<!-- from: core-model-instances4 (2026-10-22) -->
~~- **MI7 - §6.1 upward sheet pass:** what happens to initial own-instance keys before any non-I source exists (including all-I keys); does encountering a non-I source preserve the downward pass's scale? [preserve downward results until a source exists; retain scales of non-I keys because the upward pass only assigns a scale to I keys; retain opaque MI1 values]~~

**ANSWERED 2026-10-22 (re-model-instance-contracts4):** MATCHES — §6.1: leading own keys retain downward results until a nonzero non-own upward source exists; source keys retain their downward scales, including all-own input. Added I=0 failure and wrapping-key limits.

<!-- from: core-model-instances4 (2026-10-22) -->
~~- **MI8 - §4 repeated SortMeshes:** are ties stable against container order on each call or against the preceding processor permutation? [restart from container order each call, then literal descending stable insertion sort, including unordered comparisons]~~

**ANSWERED 2026-10-22 (re-model-instance-contracts4):** DIFFERS — §§4 and 4.1: restart from container order each call, but insertion is before equal and unordered scores; ties reverse container order rather than remaining stable.

<!-- from: core-model-instances4 (2026-10-22) -->
~~- **MI4 - §2 world bounds:** is the transformed model box the enclosing AABB of all eight corners, including negative uniform scale and non-symmetric matrices? [literal enclosing AABB; preserve empty boxes]~~

**ANSWERED 2026-10-22 (re-model-instance-contracts4):** DIFFERS — §2.1: full row-matrix/scale/translation interval transform encloses all endpoint combinations, handles negative scale, ignores kind, and reorders inverted bounds instead of preserving empty sentinels.

<!-- from: core-model-instances4 (2026-10-22) -->
~~- **MI5 - §2 Pick/view-utilities §7:** is nearest hit distance the signed triangle ray parameter t, or Euclidean distance to the intersection for non-unit directions and intersections behind the origin? [literal signed t; the geometry ray test explicitly accepts t < 0]~~

**ANSWERED 2026-10-22 (re-model-instance-contracts4):** DIFFERS — §2.1 and view-utilities §7: distance is Euclidean origin-to-hit length, positive even for accepted behind-origin hits; strict ordered decrease only, not signed t.

<!-- from: core-model-instances4 (2026-10-22) -->
~~- **MI9 - §§2–4 malformed/numeric inputs:** are non-finite playback/transform inputs and invalid primitive vertex/index references in scope? [preserve literal f32 comparisons/arithmetic, no FMA; unordered animation magnitude takes the one-step catch-up branch; under-sized primitive triangle counts saturate to zero and invalid mesh references are skipped for memory safety, consistent with graphics-formats' safe reader choices]~~

**ANSWERED 2026-10-22 (re-model-instance-contracts4):** DIFFERS — §§2.1, 3.1, 6.2, 9.1: x87 transient arithmetic with explicit f32 stores, unordered animation magnitude does not step; primitive subtraction is signed without saturation and iteration stops for nonpositive results; invalid-reference skips and checked parsing are portable safety choices, not native rules.

<!-- from: glue-modelinst-shadow11 (2026-10-24) -->
- ~~**MI14 - Live shadow observation and native binding:** publish a deterministic read-only route to live basic/static/animated instances, processor outputs and sprite resources, including enumeration capacity/count, ownership and safe phase. SDK GetModelsInVolume has no published capacity/ownership contract; network GetModelInstances arguments are unknown. Specify animation/processor/resource getter signatures and layouts, and reconcile SDK pointer-returning GetDrawAttributes with §1's output record. Can bounds getters refresh original caches during a strictly read-only shadow? Confirm receiver-only thiscall constructor 0x00802950/nondeleting destructor body 0x00802620 on separate 0xA4-byte fixture storage. [Only a finite isolated static constructor/getter/transform/bounds fixture is measured; no live enumeration, native private-state reads, playback, processor or sprite whole-owner credit. Constructor/body-destructor binding follows the literal receiver-only interpretation used by existing isolated renderer fixtures.]~~

**ANSWERED 2026-10-24 (re-modelinst-mi14):** DIFFERS — model-instances §§1.3, 2.2, 3.2, 4.2, 6.3, 9.2 and conformance cases 21–26: standalone constructor takes construction flag 1; only the nondeleting destructor body is receiver-only. Draw attributes require an output record and return that output pointer. Bounds getters can refresh caches. Volume enumeration is a false/no-output stub; ordinary network enumeration returns one counted pointer without capacity checking and mutates the transform, with no universal override limit. Read-only live observation therefore copies the game's existing call inputs/outputs synchronously within their lifetime, using the published animation, processor-container and sprite record/block bindings. A finite observer cap is an instrumentation choice, not a native all-owner enumeration contract; whole-owner coverage remains unproved.
