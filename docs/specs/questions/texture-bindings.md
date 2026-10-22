# texture-bindings: open questions
<!-- Questions about [texture-bindings.md](../texture-bindings.md). Append new entries at the END; when one is answered, move it to answered/texture-bindings.md. Rules: README.md. -->

<!-- from: re-texbind (2026-10-19) -->
- **TB1 - §1.1:** which interface is `0x088CEB67`, answered at `+0x04` of the binding factory? Its vtable
  (`0x00ABD8B8`) has the three `this - 4` thunks and one shared slot that returns at once.

<!-- from: re-texbind (2026-10-19) -->
- **TB2 - §1.9:** who sets the factory's display bit depth (slot 10) and its LOD offset (slot 9), and from which
  settings? graphics-formats.md §2.10 infers the texture-quality option for the LOD offset.

<!-- from: core-textures-materials4 (2026-10-22) -->
- **TB4 - §§2–3 complete owner/body inventory:** publish the missing `cS3DMaterial` methods and lifetime (including whether/how its 16-bit count and held texture bindings are released), and material-manager primary slots 6/7 (`0x0062C980` in the public ABI table). Section 3 publishes the material constructor state only and manager slots 3–5, not these bodies. Also reconcile any owner-private/destructor methods absent from §§1–3 against the requested 58-function owner inventory. [Core implements the published constructor state and manager slots 0–5 with explicit foreign system-service primitives; no material reference/binding-release or slots 6/7 contract is invented, and no blanket four-owner completion claim is made.]

<!-- from: core-textures-materials4 (2026-10-22) -->
- **TB5 - §§1.4, 1.6, 2 numeric domain:** specify operand widths, signedness, operation order and overflow/shift behavior for zero image levels, zero frames, dimensions above `0x80000000`, LOD/level shifts of at least 32, byte totals outside u32, aggregate pool/in-use counter overflow, and binding-reference overflow. [Portable core rejects zero levels/unrepresentable per-binding sizes and reference overflow, uses mathematical right shifts (zero after the word width), computes bytes with wide intermediates and aggregate counters, and accepts zero frames. These boundary choices are safety extensions pending publication, not native parity.]

<!-- from: core-textures-materials4 (2026-10-22) -->
- **TB6 - §§1.2, 1.5 keyed instance zero:** slot 6 says to cache any (group, instance), but the record uses instance zero as the unkeyed sentinel. What happens for a key whose instance is zero, including reuse, destruction and restoration? [Core rejects keyed instance zero; image-info bindings remain unkeyed. No native failure behavior is claimed.]

<!-- from: core-textures-materials4 (2026-10-22) -->
- **TB7 - §§1.5, 1.8 upload/failure boundaries:** on driver creation error, is the just-returned handle deleted as well as prior successful handles (including a zero handle)? For SetBindingImageData/restoration, publish behavior when image frame/level counts disagree with the binding, a source level is missing/null, or the upload driver reports an error; is keyed restoration traversal order observable/required? [Core includes the current returned handle in creation cleanup, stops at a missing source level or insufficient level count, ignores upload error status, and restores every keyed entry in deterministic key order. These invalid/error-domain and ordering choices are not additional native contracts.]
