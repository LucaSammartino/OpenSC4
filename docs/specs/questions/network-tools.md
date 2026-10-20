# network-tools: open questions
<!-- Questions about [network-tools.md](../network-tools.md). Append new entries at the END; when one is answered, move it to answered/network-tools.md. Rules: README.md. -->

<!-- from: the cores27 lane, the network tools core (2026-10-20) -->
- **NT6 - §3.3 step 4.1, the single-cell neighbour probe:** the spec says "Look at the four neighbours (offset
  tables `0x00AA8390` / `0x00AA83A0`)" but not the probe order, nor whether a hit stops the scan. Implemented as
  west, east, north, south in that order, all four asked (a hit only decides the axis), then the single cell
  placed through the straight path. Does the original stop at the first hit, and in which order does it ask?
  [west, east, north, south, all four *[inferred]*]
- **NT7 - §3.3 step 5.2, the join record for a same-type piece at both ends:** when the same-type probe extends
  the run at the start and the end in the same direction pass, is the join list built once per call or once per
  direction? Implemented as one `joins` list for the whole call, appended in probe order (start probe first,
  then end probe, per direction pass). [one list *[inferred]*]
- **NT8 - §10 step 3, the closure's iteration order:** the spec says "Repeat while the list grows" but not the
  scan order: does the closure rescan the whole list from the start each round, or only the new tail? Implemented
  as a tail-only walk (`k` advances, new entries append), which keeps the pointer order the removal then uses.
  [tail-only *[inferred]*]
- **NT9 - §10 step 5, the removal order of the closure's additions:** "Remove each occupant, in pointer order
  (`std::set` order)". The caller's list arrives in pointer order; the closure's additions (neighbours,
  intersection structures, paired portals) enter the vector in discovery order, which is not a `std::set`
  insertion. Implemented as: caller's order first, then additions in discovery order. Does the original sort the
  grown list before the removal walk? [caller order, then discovery *[inferred]*]
- **NT10 - §11.1 step 2.3, the refit's cell record edges:** "the type's edge mask (slot 38 with the type)" and
  "recomputed edges" — what the recomputation does is not specified. Implemented as: the record carries the
  occupant's vertex heights and the allowed ranges; the edge mask is stored but the smoothing solve reads only
  the heights and ranges (the geometry core's smoothing uses the run's edges, which a single-cell refit record
  does not carry). Does the original rebuild the edge mask from the occupant's connections before smoothing?
  [edges not needed by the solve *[inferred]*]
- **NT11 - §11.1 step 4, `FindUndergroundOccupant`'s class argument:** the spec names the class "from slot 81"
  but the piece lookup happens per solved cell; whether the original looks up by the cell's own occupant or by
  the class across the whole cell is not stated. Implemented as: per solved cell, the host's
  `find_underground_solved(x, z)` returns the cell's occupant. Is the lookup the cell's occupant (any) or the
  first of the class? [the cell's occupant *[inferred]*]
- **NT12 - §6.1 step 3.1, the crew's `&set` argument:** `crew.AddNetworkSegment(&set, …)` — whether `set` is the
  solved-cell set `+0xA4`, the solved-cell vector `+0x98` or the world-cache records is not stated. Implemented
  as: the host receives the run (cells of the valid steps) and decides. Which container does the original pass?
  [the solved-cell set *[inferred]*]
- **NT13 - §5.1 step 7, the regional retry's failure handling:** after `AttemptNeighborConnections` finds one
  and the solve is retried, a retry failure — does the original keep the first attempt's failure code or
  overwrite it with the retry's? Implemented as: the retry's code overwrites (the same `validate_and_solve`
  path). [the retry's *[inferred]*]
