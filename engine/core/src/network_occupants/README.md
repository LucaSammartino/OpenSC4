# Placement demolition helper

`placement_demolition::compute_demolition_region_for_placement_cells` owns the
ordinary-domain algorithm in `docs/specs/network-routines.md` §§2–8. Its host
performs only individual lot-manager getter, lot query, lot bounds, signed-byte
zone getter, filtered standard-cell lookup, QI/release, and final DemolishRegion
operations. There is no host region builder or whole-helper decision.

It owns signed bounds (INT_MAX lows, zero highs), ordered duplicate half-open
lot rectangles, inclusive published bounds with the extra clear border, full
x-outer/z-inner scans, missing-lookup bits and modulo-32 zone mask, the post-scan
0xFC00 barrier, cost-clear timing, the independent literal A9764C8C exclusion
predicate, eleven preview arguments, and unchanged final AL. A nonzero lookup
AL never clears earlier bits. Every non-null lookup/QI output is released
irrespective of AL. The borrowed placement-filter token is only forwarded to
initial lookups; it has no owned reference or state in the final filter.

## Portable storage and ownership

The existing `zones::CellRegion`, also re-exported by demolition, has matching
inclusive bounds and column-major logical bits. Valid city sides 64/128/256
produce region sides at most 257, below its existing 4096 storage cap. No capped
zone algorithm is changed. Native column-pointer arrays, 32-bit word allocations,
object sizes, and pointer offsets remain representation/binding limits. Tests
independently project logical cells into zero-padded LSB-first column words;
this does not claim native memory equivalence or native execution.

`RegionOutput` retains displaced boxed regions, preserving their allocation
identity rather than destroying an old output on publication. Callers can take
ownership of both current and displaced regions. Cost and optional set are
borrowed original storage, forwarded unchanged. `Rc<ExclusionFilter>` models
local creation at one reference, synchronous foreign holds, and owner cleanup.
Typed primary/serializable QI views share that allocation; native shared COM
base implementations are not provided. Predicate state is empty. Demolition's
internal filtering, pricing, sets and reference effects remain foreign.

## Qualification and limits

The eight public §8 fixture groups use independent bounds, word literals and
ordered primitive/ref ledgers, including null/non-null QI output discriminators,
noncanonical nonzero AL, final false with negative signed 64-bit cost and set
mutation, and local filter hold/release identity. Foreign preview fixtures only
check arguments, probe the owned predicate and supply explicit output writes;
they do not implement demolition or act as a whole-helper oracle.

NRT1 remains open. `Unsupported` is a portable admission result for invalid
input coordinates or malformed/out-of-city lot rectangles, never native false,
clipping or rollback. Rust slices and disjoint borrows exclude malformed ranges,
required null outputs and aliasing. Required allocation success, initialized
services, backed non-null tokens, valid foreign filter state and returning
primitives are preconditions. Allocation failures (including the native null
filter branch), reentry, exceptions and invalid-object behavior are unsupported.

This is the FIRST ordinary helper owner only. The network_rules foreign bridge
remains available; no evaluator/intersection/solve, model, record, source, C ABI,
plugin, native binding, default behavior or coverage claims are changed.
Original netocc NO14/18–20 and model NO27–31 caps remain intact. No game run or
native-original comparison qualifies this portable implementation.
