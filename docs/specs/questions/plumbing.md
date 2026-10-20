# plumbing: open questions
<!-- Questions about [plumbing.md](../plumbing.md). Append new entries at the END; when one is answered, move it to answered/plumbing.md. Rules: README.md. -->

## From the plumbing grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/plumbing.md`. Ini: `plumbing_shadow = 1` (needs our simulator and the ORIGINAL plumbing simulator:
do not enable `plumbing`); one line per pass at the original's swap and after its state 8, one per month. The
shadow's current answer is in brackets.

<!-- from: From the plumbing grid shadow (2026-10-14) -->
- **PS9 — first in-game shadow (run 81, coordinator):** in one pass (2028-03-04, pass 357) producer 0's production
  and efficiency read 0 in ours (orig 19944 and 0.997), although the pass's produced total matches (39687) and the
  grid is identical. This looks like a shadow-side read of the wrong producer record (not an RE question); the next
  plumbing lane should look at how the shadow pairs our producers with the original's.
