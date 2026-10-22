# Persistence manager independent vectors

`independent.json` is executed by `tests/persistence_manager.rs` (40 separately
named tests). The subdirectory keeps this dedicated runner separate from the
top-level simulator-vector generator. Cases PM01–PM30 and PMU01–PMU06 are the
independent inputs of persistence-manager.md §§10/10.1; EX01–EX04 cover additional
constructor/destructor, primitive failure, byte-broadcast and lock contracts.
Private core tests cover the PM19 null-only node, signed/wrapped maintenance
counter, generated-ID wrap, deque block boundaries and three-duplicate rehash.

Each case initializes foreign objects and executes `steps` in order. Resource
keys are `[type, group, instance]`. Object identities are numbers; output cells
start at 999 unless the operation supplies its own pointer. `expect` checks a
return or collection, `out` checks the pointer cell, and `release_out` releases a
successful referenced result. `refs` checks explicit logical COM counts;
`trace` checks primitive callbacks; `events` additionally checks AddRef/Release
and exact lock acquisition/release scope. `reset` clears observations, not
objects or registry state. Factory/segment/cache booleans configure the foreign
primitives, never the manager's answer. `set` changes those primitive responses.

`native_constraint` on Update cases records the unknown native stack byte S.
Only the deliberately chosen PR4 canonical initialized-OR outputs are asserted;
these vectors are specification-derived, not game measurements or native-byte
oracles. PR5's unspecified post-SetKey predicate input is not silently asserted.
