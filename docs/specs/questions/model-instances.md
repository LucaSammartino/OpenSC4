# model-instances: open questions
<!-- Questions about [model-instances.md](../model-instances.md). Append new entries at the END; when one is answered, move it to answered/model-instances.md. Rules: README.md. -->

<!-- from: re-modelinst (2026-10-20) -->
- **MI1 - §6.1:** what the five header values after a sprite animation's type tag, and the value after its five
  zoom keys, hold; nothing in the reader interprets them, and the sprite instance was not traced reading them.
  [kept, not interpreted]

<!-- from: re-modelinst (2026-10-20) -->
- **MI2 - §4:** what `LightModel`'s `flip` argument is for (it negates the rotation used to light the normals) and
  which callers pass true. [as read]

<!-- from: core-model-instances-contractfix4 (2026-10-22) -->
- **MI11 - §§2.1,3.1,4.1 numeric precision:** which x87 precision-control settings must the portable subset reproduce, and where are the binary32 stores in kind-dispatched point transforms, the ray helper and LengthOfLine? The revised contract specifies stores for bounds/score/Advance but says the transient last bits depend on active control. Is binary64 transient arithmetic an accepted portable limit or must 64-significand-bit x87 be emulated? [f64 transients for published bounds/score/playback boundaries, explicit f32 stores, no fused operations; existing point/pick vector helpers retain f32 storage. Ordinary independent vectors and a threshold whose stored accumulator rounds to 1 while its transient remains below 1 are tested; exact last-bit x87 equivalence outside those cases is not claimed.]

<!-- from: core-model-instances-contractfix4 (2026-10-22) -->
- **MI12 - §10 vector 15 second-instance stage:** does "source and second instance retain their flag and byte" mean a second frame-builder copy before that instance's own SetModel? §2 requires every SetModel with blend (8,1) to clear its own flag 0x80 and byte 0x18, so two already-bound static instances cannot both retain the original values. [test the source and a second independent frame-builder copy while SetModel mutates the first; test that independently binding either instance applies its own material changes. No shared mutable material fields.]

<!-- from: core-model-instances-contractfix4 (2026-10-22) -->
- **MI13 - §6.1 wrapped interior-zero sheet keys:** for I=0x80000001 and input keys (0,0,0x7FFFFFFF,0,0), unsigned normalization gives (I,I,0,I,I). "A zero key cannot supply a downward source" does not specify whether encountering this interior zero fails immediately, resets the source, or fails only for a later equal-zero key. Does ReadAnim succeed, and what exact scales/source zooms are written? Also specify an initially zero key at zoom 4 after wrapping. [literal conservative failure whenever a normalized key is zero; this is marked a portable choice, separate from the proved I=0 false result and nonzero unsigned-wrap vector. No exact claim for wrapped-zero native resources.]

<!-- from: glue-modelinst-shadow11 (2026-10-24) -->
- **MI14 - Live shadow observation and native binding:** publish a deterministic read-only route to live basic/static/animated instances, processor outputs and sprite resources, including enumeration capacity/count, ownership and safe phase. SDK GetModelsInVolume has no published capacity/ownership contract; network GetModelInstances arguments are unknown. Specify animation/processor/resource getter signatures and layouts, and reconcile SDK pointer-returning GetDrawAttributes with §1's output record. Can bounds getters refresh original caches during a strictly read-only shadow? Confirm receiver-only thiscall constructor 0x00802950/nondeleting destructor body 0x00802620 on separate 0xA4-byte fixture storage. [Only a finite isolated static constructor/getter/transform/bounds fixture is measured; no live enumeration, native private-state reads, playback, processor or sprite whole-owner credit. Constructor/body-destructor binding follows the literal receiver-only interpretation used by existing isolated renderer fixtures.]
