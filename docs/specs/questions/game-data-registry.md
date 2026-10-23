# game-data-registry: open questions
<!-- Native evidence boundaries from the first owner research round. -->

<!-- from: re-game-data-registry4 (2026-10-23) -->
- **GDR1 - section 2, factory and root key:** Is there a registered class-factory route for CLSID `0xEBE0E860` beyond the app direct constructor, and how is the constructor-local initial reference of the allocated root unknown balanced? The node constructor AddRefs it; the inspected registry constructor has no corresponding Release. [unknown; direct app construction, QI identities and ordinary node retain/release are specified; no substitution/factory recipe]

<!-- from: re-game-data-registry4 (2026-10-23) -->
- **GDR2 - sections 2 and 3.1, exceptional inputs:** What are the allocator/exception and null-name contracts at registry construction/creation, including partial allocations? The bodies contain null-allocation branches followed by dereferences and no ordinary transactional cleanup guarantee. [unknown; do not promise null-return rollback or accept null node parent/key]

<!-- from: re-game-data-registry4 (2026-10-23) -->
- **GDR3 - sections 4 and 5, reentry:** Which publisher/target mutations are actually supported during traversal, notification and registration replay? Native code caches raw list cells/entry pointers and does not establish snapshotting or deferred deletion. [unknown; only nonmutating vectors are normative; self/sibling/subtree deletion must not be assumed safe]

<!-- from: re-game-data-registry4 (2026-10-23) -->
- **GDR4 - section 7, diagnostics:** What exact output, destinations and callback side effects does dump visitor `00475330` produce? Slot 28 only establishes null no-op and LIFO traversal through `00474DE0`. [unknown; diagnostic text not specified]
