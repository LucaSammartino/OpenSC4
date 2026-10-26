# regional-city-records: answered questions

<!-- from: re-regional-city-records (2026-10-21) -->
- ~~**RR6 - §5.4:** slot 13 of `cISC4RegionalCity` (`GetWorkforcePercentage`, `0x00800640`) and the residential
  simulator's counterpart return the address of the float in the integer result register. Does the SDK header declare
  the method as returning `float` (a plug-in caller would then read the x87 stack and get garbage), and do all
  callers in the executable dereference the address? [as read: the interface returns an address]~~

  **ANSWERED 2026-10-26 (re-rr29):** DIFFERS — [regional-city-records.md](../../regional-city-records.md)
  §5.4 confirms both SDK float declarations and the regional slot's address return, but the residential
  method writes through a caller-provided destination and returns that pointer. The cache caller passes
  storage and dereferences EAX. A vtable-slot trace with return-consumer capture is specified to settle the
  remaining exhaustive claim about unresolved virtual callers; the native ABI itself is settled.

<!-- from: re-regional-city-records (2026-10-21) -->
- ~~**RR3 - §4.6, §4.7:** the arguments and exact effect of the network and demolition calls in the add and remove paths
  of `CreateSymmetricConnections`: the network's "place a segment between two cells" call (the last four arguments
  are `0, 0, 1, -1`), the demolition utility's region call with its occupant filter, the occupant manager's
  query-by-cell call, the per-side byte of a network occupant's connection word (mask `0x04040404`, written 2 on
  add and 0 on remove, "4 means a link to the next cell"), and the "already carries the network" test (a traffic
  network map call). Which occupants does the demolition remove, and what does the value 2 mean to the network?
  [as written in §4.6 and §4.7, marked *[inferred]*]~~

  **ANSWERED 2026-10-26 (re-rr29):** DIFFERS — [regional-city-records.md](../../regional-city-records.md)
  §§4.6–4.7 specify slot-7 straight committed placement, default tool type, typed slot-41 single-cell queries,
  and the layer-filter demolition contract in regional-city.md §6.2.1. The traffic-map test has no network-ID
  argument. Bit 2 denotes grouping; value 2 is an active low-two-bit edge type, not an asserted travel direction.
  The last matching occupant supplies the saved group mask. Worked requests/words cover add/remove and a blocker.

<!-- from: re-regional-city-records (2026-10-21) -->
- ~~**RR4 - §4.3:** the automata manager reads the boolean exemplar property `0x49BE9565` of `0x6534284A / 0xA998D30B / 1`
  into a global (`0x00B21CC1`, true in the shipped data) and `CreateSymmetricConnections` inverts the direction bit of
  every wide addition when it is false. What does the property mean (the side traffic drives on?) and does any other
  code read the global? [the spec keeps the shipped value true]~~

  **ANSWERED 2026-10-26 (re-rr29):** MATCHES — [regional-city-records.md](../../regional-city-records.md)
  §4.3 defines the verified wide-marker orientation effect and a west-pair example. The instruction export
  has only the loader argument and this read of the byte. A controlled one-property restart comparing marker
  rotations and vehicles on unchanged path resources is required to label true as right/left-hand traffic;
  no broader traffic meaning is inferred from the global.

<!-- from: re-regional-city-records (2026-10-21) -->
- ~~**RR5 - §4.3, §4.4, §4.6:** is the network numbering of the network manager exactly the table of §4.4 (network 0 the
  road, 5 the power line)? Is the odd entry of an odd number of Highway, Avenue or GroundHighway additions (an
  unpaired first half, which builds nothing) ever produced, and what does the player see if it is? Why does only the
  second half of a pair demolish and lay a piece, and does the strip of three cells starting at the inner cell (§4.6)
  cover both lanes of a wide road? [as read]~~

  **ANSWERED 2026-10-26 (re-rr29):** DIFFERS — [regional-city-records.md](../../regional-city-records.md)
  §4.4 confirms the SDK network enumeration, distinct from connection IDs. Per-connection producers accept
  singletons; an unpaired first half creates a connection/marker but no network piece or rollback. The strip
  is longitudinal and only the second half invokes the wide solver. Singleton/pair request fixtures are
  specified; tracing stock queue producers and resulting occupant cells/props is required for stock-UI
  incidence, rendered appearance and exact solved lateral footprint.

<!-- from: re-regional-city-records (2026-10-21) -->
- ~~**RR7 - §5.3:** the original and last language and country, the birth date and the world longitude, latitude and base
  elevation have setters but no function that fills them: neither the cache update nor the constructor does, and they
  are 0 in all 417 shipped records. Does any other code (the new-city dialog, the region import, a plug-in
  contract) call those setters? (The three unknown strings are RY2.) [unknown; stored and returned only]~~

  **ANSWERED 2026-10-26 (re-rr29):** DIFFERS — [regional-city-records.md](../../regional-city-records.md)
  §5.3 specifies zero construction, persistence, cache preservation and the three setter slots/addresses;
  language/country pairs have getters only. No direct setter callers were found. Nonzero metadata has a worked
  preservation/setter case. Field-write watchpoints with writer stacks during founding/import/locale changes
  are required to identify unresolved virtual or plug-in producers; zero samples do not prove their absence.

<!-- from: re-regional-city-records (2026-10-21) -->
- ~~**RR8 - regional-city.md §7.1:** who calls slot 23 (`SetupPreferences`, `0x004A78A0`) and slot 25
  (`UpdateCityCache(prefs)`, `0x004AA4F0`)? The latter sets the file path to the old directory plus the new city name
  plus `.sc4`, with no `City - ` prefix. Is that path ever used before the next save renames the file? [unknown
  callers; as read]~~

  **ANSWERED 2026-10-26 (re-rr29):** MATCHES — [regional-city.md](../../regional-city.md) §7.1 specifies
  immediate observable prefix-free path assignment, no filesystem rename, and a concrete preference/path case.
  Neither method has direct callers in the export; no native virtual caller is positively resolved here.
  Breakpoints on both methods, per-object path reads/writes and correlated file opens between assignment and
  replacement are the stated runtime observation needed to settle callers and intervening disk use.

<!-- from: re-regional-city-records (2026-10-21) -->
- ~~**RR9 - §2.1, §2.2:** `SC4WriteRegionalCity` opens the record with the auto-save stream helper's last argument set
  to 1. What does the flag mean (compress, replace an existing record)? Does the package layer rewrite the whole file
  or append when one record of an existing package is replaced, and what do the package's `+0x1C` call (the commit
  whose failure turns a good write into false) and its neighbours do? [as read; an engine may rewrite the package]~~

  **ANSWERED 2026-10-26 (re-rr29):** DIFFERS — [regional-city-records.md](../../regional-city-records.md)
  §§2.1–2.2: last argument 1 is the written u16 stream version, separate from the hardcoded OpenOStream boolean.
  +0x1C tests IsOpen; cleanup preserves the saved stream result, even on false IsOpen/Close. Replacement removes
  the old index entry/allocation before writing a new one; compression uses a separate key/size policy. R/S and
  cleanup worked cases are specified. Physical reuse/append/compaction remains bounded to an offset/hole-table/
  filesystem-write observation across small/same/large replacements, rather than a guessed DBPF layout rule.
