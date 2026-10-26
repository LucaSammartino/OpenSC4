# regional-city-records: open questions
<!-- Questions about [regional-city-records.md](../regional-city-records.md). Append new entries at the END; when one is answered, move it to answered/regional-city-records.md. Rules: README.md. -->

<!-- from: re-regional-city-records (2026-10-21) -->
- **RR1 - §6.3:** the PreSave message carries the caller's flag (data4; `cSC4App::SaveCity`'s second argument, whose
  one caller `RequestCloseCity` passes the state of the Alt key after the "save before closing?" dialog). The city
  clears its "thumbnail written" bit when the flag is non-zero, and while the bit is clear and the old file has
  picture 0 it copies the old pictures instead of rendering. What is the flag meant to be (quick save, save as, a save
  from the region view), who passes a non-zero value, and is it intended that a flagged save never re-renders the
  thumbnail while an unflagged second save of a session does? [as read: copy when the flag is set or on the first
  save after a load; render otherwise]
<!-- from: re-regional-city-records (2026-10-21) -->
- **RR2 - §6.1, §6.4:** what does the second capture pass produce (pictures 4 and 6, present only in saves with
  application version 64 or 65), what does view-manager mode `0xF3` switch off (the one picture checked shows terrain
  and streets without buildings), which region-screen map mode reads these pictures, and what are the four camera
  offsets `(1/6, 1/3)`, `(1/3, -1/6)`, `(-1/6, -1/3)`, `(-1/3, 1/6)` (four tiles of one picture, or four rotations)?
  Which test decides that the game window was inactive during the capture? Where do the other colours of the border
  masks come from (picture 2 holds 32 distinct values besides white and cyan: blending, or a third ring)? [as read: one
  picture per set, the rest unknown; an engine may draw its own]
<!-- from: re-regional-city-records (2026-10-21) -->
- **RR3 - §4.6, §4.7:** the arguments and exact effect of the network and demolition calls in the add and remove paths
  of `CreateSymmetricConnections`: the network's "place a segment between two cells" call (the last four arguments
  are `0, 0, 1, -1`), the demolition utility's region call with its occupant filter, the occupant manager's
  query-by-cell call, the per-side byte of a network occupant's connection word (mask `0x04040404`, written 2 on
  add and 0 on remove, "4 means a link to the next cell"), and the "already carries the network" test (a traffic
  network map call). Which occupants does the demolition remove, and what does the value 2 mean to the network?
  [as written in §4.6 and §4.7, marked *[inferred]*]
<!-- from: re-regional-city-records (2026-10-21) -->
- **RR4 - §4.3:** the automata manager reads the boolean exemplar property `0x49BE9565` of `0x6534284A / 0xA998D30B / 1`
  into a global (`0x00B21CC1`, true in the shipped data) and `CreateSymmetricConnections` inverts the direction bit of
  every wide addition when it is false. What does the property mean (the side traffic drives on?) and does any other
  code read the global? [the spec keeps the shipped value true]
<!-- from: re-regional-city-records (2026-10-21) -->
- **RR5 - §4.3, §4.4, §4.6:** is the network numbering of the network manager exactly the table of §4.4 (network 0 the
  road, 5 the power line)? Is the odd entry of an odd number of Highway, Avenue or GroundHighway additions (an
  unpaired first half, which builds nothing) ever produced, and what does the player see if it is? Why does only the
  second half of a pair demolish and lay a piece, and does the strip of three cells starting at the inner cell (§4.6)
  cover both lanes of a wide road? [as read]
<!-- from: re-regional-city-records (2026-10-21) -->
- **RR7 - §5.3:** the original and last language and country, the birth date and the world longitude, latitude and base
  elevation have setters but no function that fills them: neither the cache update nor the constructor does, and they
  are 0 in all 417 shipped records. Does any other code (the new-city dialog, the region import, a plug-in
  contract) call those setters? (The three unknown strings are RY2.) [unknown; stored and returned only]
<!-- from: re-regional-city-records (2026-10-21) -->
- **RR8 - regional-city.md §7.1:** who calls slot 23 (`SetupPreferences`, `0x004A78A0`) and slot 25
  (`UpdateCityCache(prefs)`, `0x004AA4F0`)? The latter sets the file path to the old directory plus the new city name
  plus `.sc4`, with no `City - ` prefix. Is that path ever used before the next save renames the file? [unknown
  callers; as read]
<!-- from: re-regional-city-records (2026-10-21) -->
- **RR9 - §2.1, §2.2:** `SC4WriteRegionalCity` opens the record with the auto-save stream helper's last argument set
  to 1. What does the flag mean (compress, replace an existing record)? Does the package layer rewrite the whole file
  or append when one record of an existing package is replaced, and what do the package's `+0x1C` call (the commit
  whose failure turns a good write into false) and its neighbours do? [as read; an engine may rewrite the package]
