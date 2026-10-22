# texture-bindings: open questions
<!-- Questions about [texture-bindings.md](../texture-bindings.md). Append new entries at the END; when one is answered, move it to answered/texture-bindings.md. Rules: README.md. -->

<!-- from: re-texbind (2026-10-19) -->
- **TB1 - §1.1:** which interface is `0x088CEB67`, answered at `+0x04` of the binding factory? Its vtable
  (`0x00ABD8B8`) has the three `this - 4` thunks and one shared slot that returns at once.

<!-- from: re-texbind (2026-10-19) -->
- **TB2 - §1.9:** who sets the factory's display bit depth (slot 10) and its LOD offset (slot 9), and from which
  settings? graphics-formats.md §2.10 infers the texture-quality option for the LOD offset.

<!-- from: core-textures-contractfix4 (2026-10-22) -->
- **TB8 - §1.12 restore hash-table state:** publish the key-table hash, bucket sizing/growth and chain insertion/removal rules needed to derive the required ascending-bucket/within-chain traversal from the initial size hint 100 and lookup history. [Portable factory owns cache/pool/accounting and restores entries referencing one record, including stale instance-zero entries, repeated aliases and record-derived reload keys. Restoration across distinct records explicitly reports Unsupported::RestoreTableOrder and makes no load/upload calls; sorted-key order is no longer presented as native parity. No provider supplies owner traversal outputs.]
