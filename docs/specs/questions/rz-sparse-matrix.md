# rz-sparse-matrix: open questions

Questions about [rz-sparse-matrix.md](../rz-sparse-matrix.md). No implementation defaults or native-exact claims.

<!-- from: re-rz-sparsematrix4 (2026-10-23), original final round 3/3 -->
- **SM1 - §9 attribution denominator:** reconcile public matrix 38/3276 with the eighteen exact Windows matrix-labeled entries; identify every remaining source attribution, folded alias and foreign body without treating public names as unique algorithms. No whole-owner credit until reconciled.
- **SM2 - §4 foreign primitive failures:** establish allocation/exception/partial mutation guarantees inside row/cell insertion, copying and erasers. The owned wrappers do not check insertion-result flags before incrementing total; this is not proof of successful allocation or rollback.
- **SM3 - §7 lifetime and missing operations:** locate actual matrix constructors, bounds/default initialization, clear, resize, copy and assignment if present. Resolve 009599a9 and conditional cleanup 0095a0d9, including payload destruction and initialization flag ownership. Record initializers 009a5bf2/009a5c36 are not matrix defaults.
- **SM4 - §3/§6 invalid pointers and iterator lifetime:** determine supported alias/invalidation guarantees after mutation, end increment and invalid input behavior. No invalid-pointer success or null/default fallback is established; output overlapping live storage is outside the current contract.
- **SM5 - §3/§4.3/§9 specialization sharing:** identify actual pointer-payload lookup body or exact alias/caller proof and complete E helper copy/allocation/error guarantees. The E owned ten-word copy/old-output order is instruction-verified in §4.3; that does not establish all specialization equivalences or foreign helper semantics.
