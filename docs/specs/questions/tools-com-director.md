# Tools COM director questions

## TD1 — Reached decoder bodies and attribution boundary

The 70-body / 31,293-byte inventory in tools-com-director.md §1 includes substantial
reached code, especially `00A6A25B` (5716 B), `00A6B918` (3773 B), `00A675E0`
(3405 B), `00407000` (2444 B), `00A69065` (2084 B), and `00A66CAC` (2078 B).
What are the input formats, exact arithmetic/widths, state transitions, buffer bounds,
error/partial-input returns, and ownership of every U row? Which rows belong to a
decoder/library rather than the director class? `00A66A31` has neighbor attribution
only. Native evidence is required; no complete-owner credit until each row has a
contract or an evidence-backed attribution correction approved by the coordinator.

## TD2 — Registered services' full slot contracts

The slot bodies, native widths, open/read/seek/close, creation/list behavior and
reference lifetime now have contracts in §5. Remaining precise dependencies:
what system-service ID and interface does `004496C0` obtain for `0094A977`'s
resource-to-path chain? What are the native semantic names of the two registered
interfaces (IDs `FCEFF549` and `3AFD825F`) and forwarded embedded-file slots?
What does `_ftol2` return/store on NaN, infinity or out-of-range estimates at
`0094AE63`? The open path checks none of these cases. Names must come from native
evidence. Decoder/header contracts remain TD1, not implied by the stream wrapper.
