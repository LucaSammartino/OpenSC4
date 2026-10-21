# Security review: external PRs 3 and 4

Reviewed heads: PR3 358bd900d9b576f926e4a8aa725c34bed30cc63d; PR4 685cd8f82d8a4d258877c4bbbf483887de301d07. PR4 includes PR3.

No merge-blocking security findings. All 41 changed paths are regular text: specs, questions, lane indexes, symbol tables and coverage metadata. No engine, executable scripts, dependencies, build configuration, symlinks or submodules change. No contributed code was executed. Scans found no private-key markers, suspicious command payloads, script URLs, instruction-override text or bidirectional controls. Symbol names and labels reproduced byte-for-byte with main's trusted generator and local export.

The documented legacy behavior includes unchecked negative indexes, path underflow and malformed stream-length overflow. Future implementations must bound accesses and explicitly reject undefined behavior instead of reproducing memory corruption. This review does not certify future runtime implementations.

Integration preserves main's newer core/shadow/replacement metadata and network-occupant specs. New owner rows are added without resetting completed work. Symbol corrections can change coverage's denominator as well as its numerator. PA3 records contradictory C:/ path vectors; clarify the overload before implementation. The contributor identifies a pre-existing symport vtable-label issue at 0x00AD1818; it remains outside this change.

Validation: question checks, regenerated indexes, tools tests, coverage and treemap generation, followed by merge-train gate. These changes add no replacement or shadow-exact claim.
