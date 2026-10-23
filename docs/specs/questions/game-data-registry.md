# game-data-registry: open questions
<!-- Native evidence boundaries from the first owner research round. -->

<!-- from: re-game-data-registry4 (2026-10-23) -->
- **GDR1 - section 2, factory:** Is there a registered class-factory route for CLSID `0xEBE0E860` beyond the app direct constructor? [unknown; direct app construction, QI identities, zero-initialized base/root-key counts and node retain/release are specified; no substitution/factory recipe]

<!-- from: re-game-data-registry4 (2026-10-23) -->
- **GDR2 - sections 2 and 3.1, exceptional inputs:** What are the allocator/exception and null-name contracts at registry construction/creation, including partial allocations? The bodies contain null-allocation branches followed by dereferences and no ordinary transactional cleanup guarantee. [unknown; do not promise null-return rollback or accept null node parent/key]

<!-- from: re-game-data-registry4 (2026-10-23) -->
- **GDR3 - sections 4 and 5, reentry:** Which publisher/target mutations are actually supported during traversal, notification and registration replay? Native code caches raw list cells/entry pointers and does not establish snapshotting or deferred deletion. [unknown; only nonmutating vectors are normative; self/sibling/subtree deletion must not be assumed safe]

<!-- from: re-game-data-registry4 (2026-10-23) -->
- **GDR4 - section 7, diagnostics:** What exact output, destinations and callback side effects does dump visitor `00475330` produce? Slot 28 only establishes null no-op and LIFO traversal through `00474DE0`. [unknown; diagnostic text not specified]

<!-- from: core-game-data-registry4 (2026-10-23) -->
- **GDR5 - section 3.1, anonymous-key collision:** What is the defined outcome when a new zero-key leaf's allocated entry address equals a raw key already present in the leaf index? The published rules replace zero with that address before indexing, but specify no collision outcome at that point. [portable draft refuses this input with tagged AnonymousKeyCollision; no invented overwrite, rollback or native false result]
