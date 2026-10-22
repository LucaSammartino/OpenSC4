# Tools COM director questions

**TD1 - Reached decoder bodies and attribution boundary**

The 70-body / 31,293-byte inventory in tools-com-director.md §1 includes substantial
reached code, especially `00A6A25B` (5716 B), `00A6B918` (3773 B), `00A675E0`
(3405 B), `00407000` (2444 B), `00A69065` (2084 B), and `00A66CAC` (2078 B).
What are the input formats, exact arithmetic/widths, state transitions, buffer bounds,
error/partial-input returns, and ownership of every U row? Which rows belong to a
decoder/library rather than the director class? `00A66A31` has neighbor attribution
only. Native evidence is required; no complete-owner credit until each row has a
contract or an evidence-backed attribution correction approved by the coordinator.
§5.6 now specifies `0094A2C1` including native tables; §5.7 specifies `0094A079`.
The internal decoder parser `00A667FE` still needs a contract. §1.1 records the
largest inspected bodies' boundary; sampled instructions are not decoder coverage.

**TD2 - Registered services' remaining dependencies**

The slot bodies, native widths, open/read/seek/close, creation/list behavior and
reference lifetime now have contracts in §5. Remaining precise dependencies:
what are the native semantic names of the two registered
interfaces (IDs `FCEFF549` and `3AFD825F`) and forwarded embedded-file slots?
What does `_ftol2` return/store on NaN, infinity or out-of-range estimates at
`0094AE63`? The open path checks none of these cases. Names must come from native
evidence. Decoder/header contracts remain TD1, not implied by the stream wrapper.
The framework request IDs `(056B906E,656B8EFC)` are now confirmed in §5.2;
header parser `0094A2C1` is now specified separately in §5.6.

**TD4 - Numeric-table reference incorrectly counted as a body**

Should the coordinator correct the 21-byte `00420036` body/owner attribution?
§1.1 proves the only data reference comes from two signed 16-bit table entries
54 and 66 at `00AF5218`, consumed as numbers by `00A68D6F` and `00A6935B–00A693B0`.
The packed dword accidentally resembles a code pointer, landing in a compare's
immediate. No direct callers exist. Proposed corrected candidate total is
69 bodies / 31,272 B; preserve the original 70-row inventory for audit. No export,
map, or attribution-tool edits were made, and no complete-owner credit is requested.
