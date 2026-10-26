# regional-city: open questions
<!-- Questions about [regional-city.md](../regional-city.md). Append new entries at the END; when one is answered, move it to answered/regional-city.md. Rules: README.md. -->

<!-- from: re-regionalcity (2026-10-20) -->
- **RY1 - §6.2:** the per-record rules of `CreateSymmetricConnections` (`0x004AA6D0`): which network, pole or
  power-line pieces each connection type builds or removes at the edge cell, what it demolishes and dezones, how the
  parity of types 3, 8 and 12 and the drive-side setting pick a direction, and which edge network flags it changes.
  The spec gives only the outline. [as read]
<!-- from: re-regionalcity (2026-10-20) -->
- **RY2 - §2:** what the record's three unknown strings (`+0xE4`, `+0x10C`, `+0x134`) hold; no getter reads them and
  the samples leave them empty. [as read]
<!-- from: re-regional-city-records (2026-10-21) -->
- **RY3 - §6.2 (RY1):** RY1 is answered in [regional-city-records.md](../regional-city-records.md) §4 (duplicates, sort,
  pairing and direction bits, type-to-network table, the add and remove sequences, the rollback of a connection that
  gets no edge piece). Please fold the outline of §6.2 into it or link it, and archive RY1. The network-manager and
  demolition arguments stay open as RR3. [answered there]
<!-- from: re-regional-city-records (2026-10-21) -->
- **RY4 - §6.2:** two statements of the pending-change paragraph can be sharpened: the flag is confirmed (1 create,
  0 remove: the incoming helper writes 1, the removal helper 0, and the shipped file `City - Kensington.sc4` holds
  remove-then-add pairs for the same cells), and when `CreateSymmetricConnections` removes "duplicate" records it
  deletes the **earlier** ones and keeps the last (regional-city-records.md §3 and §4.1). [verified; reconcile the
  wording]
<!-- from: re-regional-city-records (2026-10-21) -->
- **RY5 - §3, §5:** `GetWorkforcePercentage` (slot 13) returns the address of the float, not a float value
  (regional-city-records.md §5.4, RR6), so it does not belong with the plain getters; and §5 lists the five commodity
  doubles in their in-memory order (produced, demanded, exported, imported, cost per unit) while the file stores
  imported before exported (foundations.md §11.2). [verified; reconcile the wording]
<!-- from: re-regional-city-records (2026-10-21) -->
- **RY6 - §1:** `SC4ReadRegionalCity` and `SC4WriteRegionalCity` have a path form with the application-version gate
  (62 to 65, absent record 65, unreadable record 0) and a finish step whose failure makes the call return false
  (regional-city-records.md §2.2); and application.md §11.9 says `cSC4App::Read` returns true only when the
  regional-city read succeeded, whereas it returns true even when that read fails (regional-city-records.md §2.4).
  [verified; the application.md line is another spec's, filed here for the maintainer]

<!-- from: glue-regionalcity-shadow13 (2026-10-24) -->
- **RY8 - §5 commodity balance / foundations §18.2–18.3:** Does "computed in double precision"
  override the simulation-wide 24-bit x87 arithmetic contract? Live native records through 2032 differ
  from ordinary f64 balance arithmetic in 12/73 turbo and 10/73 normal observations (all other fields
  exact): native -11584.5830078125 versus core -11584.583251953125, for example. [Apply foundations
  §18.3 literally: load each double without rounding, round each of the two sums and the final
  subtraction to 24-bit precision; retain the double storage and return type. Please reconcile §5.]
