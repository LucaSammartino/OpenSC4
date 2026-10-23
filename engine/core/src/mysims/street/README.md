# Portable Street owner

`../street.rs` implements public `docs/specs/my-street-sim.md` §§2.1–3.9 as
a source-only owner. It does not change the existing runtime registration of
an ordinary MySim under the Street class ID. This is the first dedicated Street
implementation, within FIRST 1 of 3; it does not reopen any other family.

The caller supplies valid inherited state, distinct object/interface identities,
already-owned automaton tokens and an optional fresh helper token. The owner
sets its own raw word, tracking sentinel and helper empty-storage data. Unknown
helper bytes remain `None`; there is no guessed inherited default/refcount,
copy constructor or Street snapshot codec. The destructor consumes the portable
owner, releases its helper, delegates inherited destruction and returns the
bit-0 free decision. It does not perform a native allocation/free or reinstall
native vtables. Shutdown only delegates.

The existing `InStream`/`OutStream` traits match the inherited stream boundary
and are reused. `MySimState` uses an 8-bit state, `MySimHost` combines script
assignments and bool statuses, and `AdvisorMessage` is an eager value snapshot.
Those do not preserve Street's signed dword state, raw AL/full-EAX differences,
independent QI outputs and ordered lazy message reads. Here `Host` supplies
individual raw operations instead. It cannot return a next Street state,
attachment result or completed trip summary. Lot queries return retained tokens
only; the owner performs all reverse traversal, decisions, releases and script
outputs. Vec owns portable node/sentinel storage; no fake native frees are emitted.

Implemented methods: constructor stores/retention, Init, Shutdown, destruction
protocol, class ID, Read/Write wrappers, day/day-events/month, tracking getter,
Attach, Detach, Show/Hide, message dispatch, tool creation, and the trip update
(grid widths, rectangle, lot counts/selection, name/funding, disasters/cleanup).
State snapshots use RwLock and tracking uses an atomic; no callback executes
with a snapshot lock held. Normal callbacks return and provide valid outputs.
General reentrancy, exceptions and list mutation remain ST5.

Numeric bounds: direct real-to-int conversions accept exact binary32 dyadics
in signed-dword range. Funding additionally requires its binary64 product by
100 to be exactly binary32. Percentages accept zero counters for totals up to
4096, or positive counters not exceeding such a total when a conservative
rounding envelope proves one truncation across PC24/53/64 and RC choices.
This proves T6/T7/T9/T11 without assuming general native/f64 equivalence.
Ambiguous integer boundaries, nonfinite inputs, extreme values and unproved
extended-precision values return explicit `Unsupported::NumericDomain` after
the already-executed prefix. Invalid required pointers return a distinct
Unsupported result, never normal false or Complete. Ordinary false raw returns
continue wherever specified, including written outputs. No rollback is added.

Tests contain literal proposed public rows, not native goldens. Applicable
portable cases are L3–L7, S1–S2, D1–D3, A1–A4/A6–A16, H1–H4, M1–M9,
T1–T16, U1–U2 (56 distinct row IDs, with multiple discriminators per test).
L8–L10 exercise only the portable destructor/constructor portions; native table,
allocation and heap-free effects are excluded. A5/T17 test explicit unsupported
prefixes. L1–L2/R1–R3/L11 are not counted as implemented normal cases. Extra
controlled field perturbations test Hide's saved/current comparisons, without
claiming a general reentrant lifetime contract. ST1–ST5 remain open; no metadata
33/5373 versus 38/5481 owner-coverage claim or native shadow is made.
