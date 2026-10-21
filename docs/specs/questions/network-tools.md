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
- **NT15 - §14, the interface slot order at 16/17 (ClearOccupantHighlights / ClearCellPreviews):** the SDK
  header says "In the Windows binary, ClearOccupantHighlights is defined before ClearCellPreviews" (its
  note on slots 16/17), while network-geometry.md §1.1's Mac-name note reads the table as
  ClearCellPreviews (16) before ClearOccupantHighlights (17). The compiled table gives both slots pop 0,
  so the replacement's layout is unaffected; cFC4ISC4NetworkToolFull (glue29) declares the SDK's Windows
  order. Which of the two methods sits at slot 16 in the Windows binary? [the SDK's order *[inferred]*]
- **NT16 - §6.2 step 3 (the insert), `AsOccupant` and the reference:** the plain piece's `cISC4Occupant`
  view (slot 3) is folded — it returns `this + 8` (network-occupants.md §4.2), i.e. the same object with
  no AddRef. glue29's insert therefore releases only the `cISC4NetworkOccupant` reference from
  `GetClassObject` (run 218: clean, ref_warnings=0). Is there any caller of slot 3 that receives an
  owned reference (a QI-like AddRef inside the folded body), or is the folded view always borrowed?
  [borrowed *[inferred]* — game code treats the views as one object]
- **NT17 - §3.2, a diagonal PlaceNetworkOccupants with mask 0 (the in-game crash, glue29 run 219):** the spec's
  quirk 1 covers diagonal + mask ≠ 0 (places nothing, returns true). With mask 0 the run goes through §3.2's
  straight path; the passes refuse it (0xF0000001), but in glue29's game run the process then crashed inside
  the game's own code (SimCity 4.exe+0x9F0FA, reading 0x100) while walking the run's pieces — plausibly the
  plain un-modelled pieces the replacement inserts (the piece solver NG5 is the declared follow-up) reaching
  the game's vertex-array draw path. Does the original reject a diagonal mask-0 run before the commit (and
  with which code), or does it also reach the piece/draw path? [refused by the passes *[inferred]*]

<!-- from: core-powerline4 (engine lane, 2026-10-21) -->
- **NT18 - §9 step 2, `DeterminePolePositions` (`0x00650840`) and `PlacePoles(1)` (`0x00651560`):** the spec names
  them but gives no body: how pole cells are chosen along the drag (spacing, corners, reuse of existing poles), what
  the 48-byte pole record holds besides the cell, how `+0x384` (wire cells) is counted, what `PlacePoles`' argument
  means, and where slot 81 `CheckForReservedTiles` is called on this path (it is the only source of the power-line
  codes, and §9 step 4 reads the valid count and the code after step 2). The core keeps them as host hooks. [missing]
- **NT19 - network-geometry §11, the power-line parameter defaults:** only `+0x3A4` has a stated default (20). What
  are the defaults of `+0x3A8` (`0x098B27D0`), `+0x3AC` (`0x4A5F5953`) and `+0x3B0` (`0x0A5F596B`) when the exemplar
  lacks them, and what do `+0x3A4`/`+0x3A8` control? The core uses 0. [missing]
- **NT20 - §9 step 4 and §11 NT3, two literal readings:** (a) "When `real`, `AttemptNeighborConnections` is followed
  by `PlaceNeighborConnections`": does `AttemptNeighborConnections` also run in draft mode, and does
  `PlaceNeighborConnections` depend on its result? The core runs Attempt whenever the conditions hold and Place
  after it when real, regardless of the result. (b) In `CheckForReservedTiles`, a zoned (1) or road (3) last cell
  leaves `last` at the previous clear cell, so a line ending on a zoned cell loses that cell (v = last + 1); is that
  the original's behaviour, and what does a `CellIsOKForPlacement` result above 3 do (the core ignores it)? [ambiguous]
