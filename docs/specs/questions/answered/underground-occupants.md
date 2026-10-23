# underground-occupants: answered questions (archive)
<!-- Archive for [underground-occupants.md](../../underground-occupants.md). Lanes do not read this file. Open questions are in ../underground-occupants.md. -->

<!-- from: re-underground (2026-10-19) -->
- ~~**UG1 - §0, §2.3:** the two base classes, `cSC4UndergroundOccupant` (`QueryInterface` `0x0065A080`, `Write`
  `0x0065B140`, `Read` `0x0065E510`, `Draw` `0x0065B290`) and `cSC4UndergroundConnectionOccupant` (`0x00654030` …):
  their save records, drawing and model building, and how they use the static model descriptors and the diagonal
  ratio tables. The pipe and subway records cannot be written without them.~~
  **ANSWERED 2026-10-21 (re-ugbase):** network-occupants.md §3.1–§3.6 now give both base classes: the underground
  occupant's fields, slots, `Init` forms, ceiling, ratio tables, neighbour rules, `Refresh` and draw passes (§3.1,
  §3.2), its record with every field named (§3.4), and the connection occupant's fields, record and behaviour (§3.6).
  The model builders' exact geometry is NO34.

<!-- from: re-underground (2026-10-19) -->
- ~~**UG2 - §2.2:** what network flag bits `0x02000000` and `0x04000000` (`+0x13C`) mean; either one makes an
  occupant's sort value its piece id instead of its class ID.~~
  **ANSWERED 2026-10-21 (re-ugbase):** they make the network occupant draw the first and the second triangle of its
  textured quad (`0x1000000` choosing the diagonal); `SetQuadDraw` sets both for surface pieces and the underground
  base sets them from the piece id and rotation (main slot 88). So the sort value is the piece id exactly when the
  surface quad is drawn (network-occupants.md §2.11, §3.2).

<!-- from: re-underground (2026-10-19) -->
- ~~**UG4 - §4:** the values `0x2A183167` (pipe connection) and `0x8A13918A` (subway connection) that `Init` stores at
  `+0x24` before the base `Init`.~~
  **ANSWERED 2026-10-21 (re-ugbase):** `+0x24` is the occupant type that occupant slot 7 `GetType` returns: the
  connection classes derive straight from `cSC4Occupant`, so each subclass's `Init` writes its own type there
  (network-occupants.md §3.6).
