# network-type-info: open questions
<!-- Questions about [network-type-info.md](../network-type-info.md). Append new entries at the END; when answered move to answered/network-type-info.md. -->

<!-- from: re-network-type-info4 (2026-10-23) -->
- **TI1 - Present property variant domain:** What do variant slots +0x94/+0x64/+0x28 and raw-storage/count slots return or write for wrong-type, empty, scalar-in-vector and vector-in-scalar properties? §5 proves owner destinations, absence preservation and fallback branches, not coercion/rejection or partial-output safety. [unknown; keep these primitive results explicit in conformance vectors]

<!-- from: re-network-type-info4 (2026-10-23) -->
- **TI2 - Nonreturning failure and allocation:** Which allocator/null-resource/parser paths can throw, crash or return, and what unwind ownership is performed? §4.1 proves the null evaluator is still passed to AddRules; no owned rollback is visible. First/later normal false-result publication is specified, but nonreturning failures have no fabricated final loaded state or retained-reference result. [unknown; no safe OOM or exception guarantee]

<!-- from: re-network-type-info4 (2026-10-23) -->
- **TI3 - x87 environment and derived bits:** What precision/rounding control applies when 0x0063D460 runs, and what are exact FPTAN outputs near poles/nonfinite inputs? Straight and diagonal binary32 stores share an x87 intermediate, so the prior rounded-straight formula is not established at all precision settings. [operation/store order known; nonzero derived literal words unresolved]

<!-- from: re-network-type-info4 (2026-10-23) -->
- **TI4 - Support duplicate-key primitive:** On repeated direct LoadSupports calls or duplicate enumeration keys, does ReadSupportStructureInfos preserve, replace, append or reject an existing entry and which references are freed? §4.2 specifies only the owner forwarding and outer Clear boundary; parser/map internals are outside this lane. [unknown; no deduplication or accumulation policy inferred]
