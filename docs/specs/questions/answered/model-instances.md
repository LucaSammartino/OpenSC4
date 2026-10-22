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
