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
The internal decoder parser `00A667FE` is specified in §5.9. §1.1 records the
largest inspected bodies' boundary; sampled instructions are not decoder coverage.
2026-10-22 residual phase: §5.8 now fully specifies `00A675E0` (3405 B), including
bit extraction, both rate layouts, partial mutation and retained fields. §1.1 adds
precise synthesis/history/input/output boundaries for `00A6A25B`/`00407000`, whose
complete coefficient arithmetic remains open. §§5.9–5.11 additionally specify the
internal header, resets, ring bits, initialization, dispatch, PCM store and planar
drain contracts. Final phase §5.12 specifies all5716 B of `00A6A25B` body-local
window arithmetic, both coefficient layouts and the centre multiply-high rule;
its called transform `00407000` remains unproved. Remaining U:17 bodies/17,137 B;
exact dependencies are retained
in the spec rather than removed from the attributable scope.

**TD4 - Numeric-table reference incorrectly counted as a body**

Should the coordinator correct the 21-byte `00420036` body/owner attribution?
§1.1 proves the only data reference comes from two signed 16-bit table entries
54 and 66 at `00AF5218`, consumed as numbers by `00A68D6F` and `00A6935B–00A693B0`.
The packed dword accidentally resembles a code pointer, landing in a compare's
immediate. No direct callers exist. Proposed corrected candidate total is
69 bodies / 31,272 B; preserve the original 70-row inventory for audit. No export,
map, or attribution-tool edits were made, and no complete-owner credit is requested.
