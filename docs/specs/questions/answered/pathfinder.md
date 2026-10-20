# pathfinder: answered questions (archive)
<!-- Archive for [pathfinder.md](../../pathfinder.md). Lanes do not read this file. Open questions are in ../pathfinder.md. -->

## From the pathfinder core (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/pathfinder.md`. All fourteen §11 tests pass as unit tests and vectors `pf11_01`–`pf11_14`; extra
vectors `pf11_15`–`pf11_28`. The core's current answer is in brackets.

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF1 — §4.9 step 5:** "not g < node.g → ignore" contradicts its note that a NaN g passes both tests.
  [`node.g <= g → ignore`, so a NaN g passes]~~ **ANSWERED 2026-10-14** from the exe (`AddTripNode`
  `0x006D9070`, `0x006D9085`) and written into §4.9 step 5, with a table. **The bracket is right.** Both tests are an
  ordered `node.g ≤ g`: `fcomp`, then only C0 is tested, and an unordered result sets C0. So the node is taken when
  `g < node.g` or when **either** value is NaN; a stored NaN `g` is replaced by any `g`. The old wording was wrong.

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF2 — §4.17:** which of the record's two arrays do the "16 counters" go to? [`current`]~~ **ANSWERED
  2026-10-14** from the exe (`ApplyTraffic` `0x006DA2D3`, `0x006DA324`–`0x006DA330`; edge-density `operator[]`
  `0x006D7820` returns the value at node `+8`) and written into §4.17 step 3, test 4 and §10 Q3. **The bracket is
  right: `current`**, the record's second array at `+0x22`; `last` and `usage` are never written here. Also
  corrected: for style 2 the key-bit-1 record gets the **mirrored** index `4·exit + entry`, like `ApplyRoute`. The
  spec said "not mirrored" (also fixed in traffic-simulator.md §4.5).

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF3 — §8:** which value does the pathfinder pass for `GetLot`'s bool? [left to the host]~~ **ANSWERED
  2026-10-14** from the exe and written into §8 and §4.6. **Always false** (`push 0` before slot 6 at `0x006D8B1D`
  lot start, `0x006D7AC7` filter-0 goal, `0x006D8688` §4.5 step 5). The bool is `originOnly` (zones.md §8), so any
  cell of a lot finds it. The host should pass false.

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF4 — §4.8.3 step 3:** `conn[t]` is shifted by the node's entry side `s`, not the loop's `s2`. [literally
  `s`]~~ **ANSWERED 2026-10-14** from the exe (`FindPath` `0x006D99D1`–`0x006D99E2`) and written into §4.8.3 step 3.
  **The bracket is right: `s`**, the popped node's own side, re-read in each pass of the `s2` loop. The test does
  not depend on `s2`, so all four nodes are added or none. The branch stays unreachable (no code sets wormhole
  bit 1).

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF5 — §4.6 fallback:** several links of one connection match the start cell: which link cost? [the
  first]~~ **ANSWERED 2026-10-14** from the exe (`CreateStartNodes` `0x006D8B54`–`0x006D8F8E`; terminal layout
  from `DoFerryTerminalActivated` `0x0072392C`–`0x0072398B`) and written into §4.6. **The bracket is wrong: the
  last.** Nothing ends the loops and nothing checks for an existing node, so each matching link overwrites `g`/`f`
  of the same start node and pushes it again (the heap holds it once per match). Also corrected: the loops run over
  the **ferry terminals** and their neighbour links (traffic-network.md §2.6), not over neighbour connections, and
  the start node is at the terminal's **arrival cell**.

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF6:** neighbour-connection link lists and `GetTrafficNeighborConnection` costs are not in the traffic network
  core. [host methods]~~ **ANSWERED 2026-10-14** from the exe (`0x0070F970`, `0x007104A0`, `0x007114E0`, Add
  `0x007153C0`, Remove `0x00713AB0`, sweep `0x007138B0`, capacities `0x00714E60`, roll `0x00714BE0`) and written
  into traffic-network.md §2.7.1–§2.7.5. **The bracket is right: host methods** of the traffic simulator, now
  specified. The "link lists" are the ferry terminals' neighbour links (PF5, §2.6). The cost is
  `max(cost, f)` per Add (f = the other side's `0xCA947077` average trip length, else commuteMaxTime), never
  lowered. The lookup never fails, so pathfinder.md §4.13 step 3 always adds it. Also corrected: the per-travel-type
  arrival counts go on the connection's own holder, not the pool (traffic-simulator.md §4.2).

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF7:** do exit nodes, closed without expansion, advance the work counter? [yes; test 13 holds either way]~~
  **ANSWERED 2026-10-14** from the exe (`FindPath` `0x006D9540` → close `0x006D9C81`, counter `0x006D9C97`) and
  written into §4.11. **The bracket is right: yes.** Every popped node that is not a goal reaches the one close,
  which advances the counter when a max work time is set; an off-map node jumps there without expansion. A goal
  node does not count, and with max work time 0 the counter stays 0.

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF8 — §4.18:** which cell gives an exit node's H when its score is recomputed? [its own off-map cell]~~
  **ANSWERED 2026-10-14** from the exe (`RecalculateNodeScores` `0x006D67FF`/`0x006D6825`; key decode `0x006D5DF0`)
  and written into §4.18 (with an example) and §4.7. **The bracket is right: its own off-map cell**, the node's
  stored cell, decoded from the key (low bytes zero-extended, plus `dir(side)` for bit 31). It differs from the
  in-city cell AddTripNode used, so the recomputed `H` is usually off by one. Also noted in §4.8.4: the park-and-ride
  target is never off-map (the old "lands on P" remark was unreachable, and wrong at the −x/−z edges).

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF9 — §4.9:** Update of an open node whose heap position is stale. [no-op unless the heap slot holds it]~~
  **ANSWERED 2026-10-14** from the exe (`UpdateNode` `0x006D6460`, `Push` `0x006D88B0`, `Pop` `0x006D6380`) and
  written into §4.7 ("Position"). The original does **not** check: it sifts up from the slot the node was last
  written to and writes the node at the end, overwriting whatever the slot held. But that case never happens in
  the game: the only open node with a stale slot is the one being expanded, and no move produces its own key.
  **The bracket stands as a [choice]**; it is equivalent in every reachable flow.

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF10 — §4.15:** `GetNeighborConnectionInnerCell` fails for an off-map node. [the node's own cell, low
  bytes]~~ **ANSWERED 2026-10-14** from the exe (`0x0070DE01`/`0x0070DE10`; `GetCompressedRoute` `0x006D71E2`,
  `0x006D7550`) and written into §4.15. On failure (a corner cell) the function writes nothing, and the original
  emits the low bytes of two uninitialised or stale stack variables. **It cannot happen in the game:** the root is
  always in city and jumped-to off-map nodes are ferry exits with a valid inner cell. **The bracket stands as a
  [choice].**

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF11:** FindPath on an object that is not initialised. [returns 0, does nothing]~~ **ANSWERED 2026-10-14**
  from the exe (`FindPath` has no initialised test; constructor `0x006D89C0` leaves W, H and the table bases unset;
  `Shutdown` `0x006D5D63` nulls the bases; first write `0x006D6060`) and written into §4.3. The original faults or
  writes through garbage in `SetupDistanceTables`. It is unreachable (`CreatePathFinder` always runs `Init`).
  **The bracket stands as a [choice].**

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF12:** `LengthOfLine` precision. [the exact square root, rounded once to float]~~ **ANSWERED 2026-10-14**
  from the exe (`0x005D7660`: three `fsub`, three `fmul`, two adds, `fsqrt`, all x87 at 24-bit precision; called
  from `DistanceToRectangle` `0x006D5F01` etc. with `y = 0`) and written into §4.12 with examples. **The bracket is
  right.** Every step before the root is exact for cell coordinates, and `fsqrt` under 24-bit precision control is
  correctly rounded.

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF13:** a switch nibble naming a travel type above 8. [ignored]~~ **ANSWERED 2026-10-14** from the exe
  (`FindPath` `0x006D97D1`–`0x006D97DD`) and written into §4.8.2 (b). The only gate is bit `t2` of the u16 allowed
  mask, and no game caller sets a bit above 8, so the original ignores such a nibble in practice; with such a bit
  set it would index its 9-entry tables past their ends. **The bracket is equivalent and stands as a [choice].**

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF14:** distance-table indices outside −1…W / −1…H (unreachable). [the formula]~~ **ANSWERED 2026-10-14**
  from the exe (`H` `0x006D6160`, `SetupDistanceTables` `0x006D6040`, `RecalculateNodeScores` `0x006D6825`) and
  written into §4.4 ("Index range"). **Confirmed unreachable:** every cell whose `H` is read is in city or one step
  off it. The original has no check and would read the neighbouring heap. **The bracket stands as a [choice].**

<!-- from: From the pathfinder core (2026-10-14) -->
- ~~**PF15:** the entry side of a ferry neighbour-link exit node. [`EdgeTo(off → edge)`; assumes adjacent
  cells]~~ **ANSWERED 2026-10-14** from the exe (`FindPath` `0x006D9B55`–`0x006D9BA5`) and written into §4.8.3
  step 4, with an example. **The bracket is right:** the call passes the in-city edge cell, side `EdgeTo(off →
  edge)` and the exit flag, so the node lands one step from the edge cell towards the off-map cell. Every link the
  game makes has adjacent cells (the edge cell is `GetNeighborConnectionInnerCell(off)`), so that is the off-map
  cell; only a damaged save could break it.
- Deviations from the compiled interface (choices): `SetStartingCostArray` copies the 9 floats; `SetRouteInfo` keeps
  its own route list; the process-wide work-check values are one shared cell.
