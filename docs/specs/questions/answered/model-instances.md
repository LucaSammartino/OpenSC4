# model-instances: answered questions

<!-- from: core-model-instances4 (2026-10-22) -->
~~- **MI3 - §§1–2, constructor/records:** what are the base/static initial owner, custom data, properties and draw attributes; which bytes in the 20-byte draw-attribute record hold flags; and what is the transform change counter's width and SetTransform increment source? [Rust accepts explicit base defaults; static custom data/properties start zero, supplied draw attributes; semantic flags plus 16 opaque bytes without native packing; wrapping u32 instance-local counter, never an ABI layout]~~

**ANSWERED 2026-10-22 (re-model-instance-contracts4):** DIFFERS — §§1.1–1.2 and 2: both properties default to 2; draw flags occupy byte 0, bytes 2–3 are unwritten; transform counters are bytes, Get increments destination and Set increments instance, ignoring source counter.

<!-- from: core-model-instances4 (2026-10-22) -->
~~- **MI10 - §1 reference lifecycle:** what is the initial native reference count and the behavior when its 16-bit counter overflows? [a returned Rust Reference owns one count; exposed arithmetic wraps at 16 bits, while backing Arc storage remains alive until all Rust handles are released, avoiding invalid memory access on counter overflow; ordinary non-overflow releases destroy the most-derived owner at zero]~~

**ANSWERED 2026-10-22 (re-model-instance-contracts4):** DIFFERS — §§1.1 and 2: constructors start at zero; basic has wrapping 16-bit count, static has wrapping 32-bit count; Release destroys at either zero or one, including after overflow. Safe surviving backing storage is a portable choice, not native lifetime.
