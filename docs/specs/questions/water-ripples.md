# water-ripples: open questions
<!-- Questions about [water-ripples.md](../water-ripples.md). Append new entries at the END; when one is answered, move it to answered/water-ripples.md. Rules: README.md. -->

<!-- from: re-ripples (2026-10-19) -->
- **WR2 - §1.2:** what the terrain vertex flags `0x10` and `0x01` mean (shore and water are inferred), and which side
  of the traced shore the normal N points to.

<!-- from: re-ripples (2026-10-19) -->
- **WR3 - §2.1, §2.2:** the meaning of the three bytes b1–b3 of a textured selection and of its modes 1–3.

<!-- from: core-terrain-ripples4 (2026-10-22) -->
- **WR8 - owner inventory/defaults/lifecycle:** the brief counts 65 functions/15308 bytes, but the spec and public SDK do not enumerate the owner API. Publish a complete per-owner method checklist with signatures, constructor/destructor and Init/Shutdown/Cleanup semantics, initial fields/refcount/return values, repeated/failed Init behavior, tuning absent-property/type behavior, and texture/factory acquisition/release timing. [Rust Arc/Drop and explicit shutdown clean all held references; tuning retained after first successful Init. These are safe core policies, not native lifecycle claims. No complete-owner credit.]
  **PARTIAL 2026-10-23 (re-ripples-contracts4):** §§2.4–2.5 supersede the older partial note: actual vtables/reference/deleting paths, zero constructor refcount, no destructor Shutdown, process flag reset only, retained globals, unchecked required-service failures, repeated Init behavior, acquisition/release and unsafe texture failure are specified. §2.6 binds the ripple scalar callers to the already published data-formats §5.1 converter, including numeric tag/target gates, output-on-success, temporary target mutation, unsigned clamp and independent absence/reset/raw-array ledgers. §2.7 specifies bounded nested storage signatures/copy depth/alias domains/destruction order and selection record insertion/cleanup. **Remaining open:** complete owner membership and remaining helper signatures beyond that bounded ledger; 65 functions/15308 bytes versus the address-window 69/15276 attribution remains unreconciled; Pick's four semantic argument types remain opaque 32-bit words with false/no-output behavior, not a typed native-call recipe. Unproved foreign text/object/exception and allocator/error/reentrancy cases retain their typed foreign boundary; no full foreign conversion matrix, rollback or coherent-Drop guarantee is inferred. Do not archive WR8 as wholly answered or promote these contracts to complete-owner/native/runtime/byte coverage. WR1–WR3 remain open independently.

  **PARTIAL 2026-10-23 (re-ripples-wr5):** §2.5 adds direct ripple argument/result/receiver checklist and reproducible inventory split (selection window 25/5838, ripple window 44/9438). RemoveLastRipple is a strip-list operation, not a ripple-object receiver method. Historical 65/15308 has no supplied membership manifest and remains unreconciled; foreign/error domains in the preceding note remain unproved. WR8 is deliberately not archived as wholly answered. WR1 and WR11–WR14 are now independently answered.


