# renderer-gdrivers-director-inventory: open questions
<!-- Engine questions; append only. -->

<!-- from: core-gdrivers-director4 (2026-10-22) -->
- **GI1 - §1 constructor subobjects:** the constructor prose specifies zeroed matrix storage without its extent, and initializes temporary/working storage without placing aggregate construction among the ordered allocation steps. Is the aggregate constructed between pool construction and working-array allocation? [Literal portable model exposes zero initial matrix words without a guessed shape, and constructs the aggregate after the pool and before the two working arrays; unspecified state-word bits remain unknown, with only bit 0x2000 known clear.]

<!-- from: core-gdrivers-director4 (2026-10-22) -->
- **GI2 - §§1.2,5.7 pool storage:** what descriptor/set allocator policy and virtual deallocation arguments are observable? [General container allocation internals are outside §8's claim. Core builds the exact ordered logical descriptors after virtual allocation, tracks optional foreign backing storage for ordered cleanup, and distinguishes virtual from heap freeing without claiming a native release-flag contract.]
