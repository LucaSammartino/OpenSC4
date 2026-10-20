# power: answered questions (archive)
<!-- Archive for [power.md](../../power.md). Lanes do not read this file. Open questions are in ../power.md. -->

## From the coordinator (run 61, 2026-10-14)

<!-- from: From the coordinator (2026-10-14) -->
- ~~**CR1 — power.md §13 Q2: who posts ConnectionsReady `0x6AC284F3`, and what triggers it?** In run 61 it never
  arrived after loading Konradshohe.~~ **ANSWERED 2026-10-14** from the exe and written into foundations.md §11.6,
  with pointers in power.md (§4 table and §13 Q2), plumbing.md, neighbors.md and advisors.md §6.8.1.
  - The only poster is `cSC4RegionalCity::CreateSymmetricConnections` (`0x004AB778`): queued, no data, on
    TerrainReconciliation for the current city.
  - TerrainReconciliation comes from the terrain's second animation tick after its PostCityInit, so once per load.
    The player's reconcile-edges command also posts it.
  - With the default preference the tick defers to `ReconcileEdgesWithNeighboringCities`. That function waits while
    a modal window is open and **posts nothing while popup dialogs are disabled** (`SetPopupDialogsEnabled(false)`).
  - None of land value, weather, ordinance or demand appears in this chain.
  - **Confirmed in game** (coordinator): plumbing alone stays at `enabled=0` with `popup_dialogs = 0` set at
    PostAppInit (run 64) and is enabled every month without it (run 65). The plugin now sets the flag only at
    ConnectionsReady (`d161d20`, the §11.6 [choice]); run 66 has both: no popups, plumbing enabled all 38 months.

## From the power grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/power.md`. Ini: `power_shadow = 1` (needs our simulator and the ORIGINAL power simulator: do not
enable `power`); one line per recompute cycle at state 3, plus month, bookkeeping and month-summary lines. The
shadow's current answer is in brackets.

<!-- from: From the power grid shadow (2026-10-14) -->
- ~~**PW1:** `FindUnvisitedConnection` is not in the spec. The shadow takes the first connection from
  `GetConnectionsOfTypeToCity` whose visited flag is false. The original's search order? [first unvisited]~~
  **ANSWERED 2026-10-14** from the exe (`0x006CF720`): **the bracket matches.** It walks the connection list from
  the head in append order and returns the first with type `t`, visited bit clear and other-side city `city` (no
  AddRef), the same order as `GetConnectionsOfTypeToCity`. Written into power.md §6.5.

<!-- from: From the power grid shadow (2026-10-14) -->
- ~~**PW2:** the log date for SimNewDay, SimIdle and SimNewMonth comes from the message's `data1` as a julian date.
  Does SimIdle's `data1` carry the date? [assumed]~~ **ANSWERED 2026-10-14** from the exe (`SimulateIdleTime`
  `0x00702E38`, `SimulateAgents` `0x007038E7`): **the bracket matches.** Both read the simulator date's number
  (`GetSimDateNumber`, julian day) once per call and pass it as `data1` of every agent message, SimIdle included.
  Written into power.md §4.3.

<!-- from: From the power grid shadow (2026-10-14) -->
- ~~**PW3:** the pass import, export and demand are not saved. What does the original hold after a load, before its
  first state 0? [counted as `not_exact` until then]~~ **ANSWERED 2026-10-14** from the exe (constructor
  `0x006EF805`–`0x006EF80B`; `Read`, `Init` and `Write` never touch them): **0.** The bracket is safe but not
  needed: the shadow can start them at 0 and compare exactly. A save taken in state 2 or 3 makes the original's
  state 3 publish only the post-load share (all 0 for a state 3 save). Written into power.md §9.1 with an example.

<!-- from: From the power grid shadow (2026-10-14) -->
- ~~**PW4:** `brokenFraction` is not saved; the shadow starts it at 0. The original's value after a load? [0]~~
  **ANSWERED 2026-10-14** from the exe (constructor `0x006EF8A7`; written only there and in `ApplyPoleFunding`
  `0x006EC3F0`): **the bracket matches.** It is 0 after a load, while the saved brokenCount is kept, so the first
  month repairs poles down to the new target (all of them at 100 % funding). Written into power.md §7.6.

<!-- from: From the power grid shadow (2026-10-14) -->
- ~~**PW5 — §5.3:** the dirty rectangle is not saved; the shadow uses the bounding box of the changed wire bits. Can a
  larger rectangle give different wire bits?~~ **ANSWERED 2026-10-14** from the exe (`UpdateWireBits`
  `0x006ED6E0`): **yes, through zones only.** The occupant query has a 2R margin, but the zone pass re-wires only
  zoned cells inside the rect. A cell of the smaller rect wired only by a zoned cell that only the larger rect
  holds loses its bit with the smaller one. *[choice]*: take the rect our simulator merges from the same messages.
  Written into power.md §5.3 with an example.

<!-- from: From the power grid shadow (2026-10-14) -->
- ~~**PW6:** the shortfall block probe tries starts 0 to min(count, 32768), assuming one `rand()` of at most 0x7FFF.
  True for networks with more than 32768 blocks?~~ **ANSWERED 2026-10-14** from the spec's block rectangle and the
  C `rand()` (foundations §12): such networks cannot exist. The block count is at most (256 / 8)² = 1,024, and the
  start is one `rand() mod count` with `rand()` ≤ 0x7FFF, so the probe range is right and covers every start.
  Written into power.md §6.3.

<!-- from: From the power grid shadow (2026-10-14) -->
- ~~**PW7 — §7.5:** the ordinance factor is read live when the shadow starts, then each month. Does the original read
  it anywhere else (at load, on an ordinance change)?~~ **ANSWERED 2026-10-14** from the exe (writers `0x006EF889`,
  `0x006ED6B2`, `0x006EC580`): **at load yes, on an ordinance change no.** The tuning read in `Init` (so also after
  a load) and on `0xCA1B151C` reads it, and so does each month start. `OrdinanceUpdated` is handled only for the
  Nuclear Free Zone, so a mid-month enactment takes effect at the next month. Written into power.md §8.
- Not compared (cannot be read without changing the game): posted messages, history values, budget line items,
  demolitions and lot conditions, which poles the month's shuffle breaks (the broken count is compared), deal
  flags, the dirty rectangle.

<!-- from: From the power grid shadow (2026-10-14) -->
- ~~**PW8 — first in-game shadow (run 80, coordinator):** all 213 cycles and 94 month lines are exact, but every
  bookkeeping line differs a little: the original's running counts against our from-scratch insertion of the live
  occupants, e.g. consumers 725/722, requirement 6481/6480, 4 consumption-map cells (max |d| 2). Which occupants does
  the original count that a rebuild from the live occupants does not (or the other way round)?~~
  **ANSWERED 2026-10-14** from the exe (`0x006ECF2D`, `0x006EC25F`: values read at delivery; nothing rebuilds the
  counts) and a recount of the shipped saves: **no live occupant.** The persistent +2 (cells (41,102) and (55,111))
  is in the Konradshohe save itself. The saved counts exceed its own buildings by the mayor's statue and the
  cemetery, which consume 0 in the shipped exemplars (Tegel: the same at its cemetery). The rest are posted
  occupant messages not yet delivered at the month. Written into power.md §5.2 with a test, and the removal's
  `initialized` gate added to §5.

## From the power shadow under the turbo clock (filed by the coordinator from the `powerturbo` lane's report, 2026-10-14)

The power shadow's bookkeeping lines were exact at normal speed but not under `sim_turbo` (regress gsT1: 306 of 352).
The cause was the shadow's classifier, not the power core: building messages still in flight at a month line, which
turbo makes more frequent. The lane fixed it in the shadow (exact under turbo and at normal speed) and implemented
the brackets below, marked *[choice]* in code.

<!-- from: From the power shadow under the turbo clock (2026-10-14) -->
- ~~**PW9:** power §5.2 takes the first bookkeeping line as the baseline of differences inherited from the save, but
  that line can have building messages in flight. Should the spec say how the baseline is chosen? [the first line
  equal to its predecessor; never edited afterwards; an entry that reads 0 is listed as absent and counts as
  inherited again when it returns; a non-inherited difference is a mismatch only when it is unchanged at a line
  where nothing else changed, or unchanged across three lines]~~ **ANSWERED 2026-10-15 (re-answers10):** glue, accepted, with a better rule now in the spec. A posted occupant message reaches every subscriber in the same `MessageSend`, so a shadow subscribed to `0x99EF1142`/`0x99EF1143` has seen exactly the messages power has handled. The occupants in flight are those whose listing in the occupant manager differs from the delivered messages. Recommended: build the expected counts from the delivered messages, which removes case 2. Otherwise take the baseline at the first line with nothing in flight. The bracket's heuristic is accepted. power §5.2.

<!-- from: From the power shadow under the turbo clock (2026-10-14) -->
- ~~**PW10:** given §5.3 (PW5: a partial rebuild leaves bits set outside the rebuilt area, and no insertion filters
  run during a rebuild), should the bookkeeping line compare wire bits at all? [listed as "wire bits, not counted";
  kept out of the baseline and the mismatch count. The cycle lines still compare every rebuild's wire bits exactly.
  No counted check then covers the wire bits that §5.1 insertion sets directly]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES (accepted): the original's wire bits depend on history. Insertion sets them with filters; a rebuild clears and re-wires only the dirty rect, without filters and with no margin for zoned cells; bits outside a rebuilt rect only accumulate. So no counted comparison against a rebuild is exact. Only a mirror that replays the same sequence covers the §5.1 bits. power §5.2.

<!-- from: From the power shadow under the turbo clock (2026-10-14) -->
- ~~**PW11:** under turbo the baseline can settle late (the 9th line in one run), so a real difference that appears
  earlier is taken as inherited; and a frame that opens two month days keeps the first day's messages in flight at
  the second, which then looks settled. Is a count of queued occupant messages (from the message server) a
  documented, reliable signal that the shadow could use instead of this guesswork? [not used]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES (not used). The queue count exists: `cIGZMessageServer2` slot 10 (`0x0092D0F4`) returns the number of queued messages, excluding the one being delivered. But it counts every type and is never 0 at a month line, because the day's SimNewDay/SimNewMonth are queued in the same tick. The reliable signal is the delivered occupant-message stream (PW9). power §5.2.
