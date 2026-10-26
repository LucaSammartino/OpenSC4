# regional-city: answered questions

<!-- from: replace-region27 (2026-10-26) -->
- ~~**RY9 - §6.2 / regional-city-records §4.6–§4.7 (RR3), replacement blocker:** What `privilege`, `flags`, `dezone`
  and caller-filter settings does CreateSymmetricConnections pass to `DemolishRegion` for its border strip?
  The regional-city-records §4 introduction explicitly leaves the exact native arguments open as RR3; §4.6 says
  no confirmation/no charge and gives a filter layer, but does not select these inputs. demolition.md §4.4 makes
  them observable: privilege 0 tests removal level 1 (system lock), nonzero tests level 2 (player lock), and nonzero
  privilege with flags bit 0x2000 clear also applies the current view filter. Please specify the caller filter's
  accepted occupant types as well as its layer. Network placement itself is documented in network-tools.md §3 and
  network-manager.md §9; that part needs no new contract. The portable core has the semantic border sequence, but
  native registration remains unimplemented until this demolition host path can be wired without choosing unknown
  removal/view behavior. The lane is stopping this item rather than guessing these inputs.~~

  **ANSWERED 2026-10-26 (re-ry28):** MATCHES — [regional-city.md](../../regional-city.md) §6.2.1 and
  [regional-city-records.md](../../regional-city-records.md) §4.6 specify actual demolition, privilege 0, flags
  `0x1`, dezone false, a view-level caller filter (surface 1, Subway 2, Pipe 3), null output/clicked pointers and
  zero final coordinates. The caller's type-only query admits every type; its occupant query matches the
  classified layer. The utility applies its own type gate and system-lock test, omits its occupant view filter,
  but retains the current-view effect/empty-lot gates and network view-level check. A masked west-edge request
  demonstrates player-only versus system locks and hidden flora. No placement contract is changed.

<!-- from: core-regionalcity9 (2026-10-24) -->
- ~~**RY7 - foundations §11.2 / regional-city §2, Read edge cases:** Does Read clear the constructor's
  commodity entries 1–3 before loading a map (foundations §14.20 requests a byte-identical round trip with
  two commodities)? What do negative signed commodity and deal counts do? Are object-level Read failures
  atomic, and are duplicate map/POI keys last-wins? The core wire decoder replaces maps (last key wins),
  rejects a commodity count interpreted unsigned above the specified limit, treats negative deal counts
  as an empty signed loop, and publishes a new object only after successful decoding. These are literal
  readings; no additional failure behavior or default entries are inferred for a decoded wire map.~~

  **ANSWERED 2026-10-26 (re-rr29):** DIFFERS — [foundations.md](../../foundations.md) §11.2 and §14.20,
  [regional-city.md](../../regional-city.md) §2: native Read retains constructor/default and other absent
  keys, overwrites duplicate scalar-map values, concatenates duplicate POI lists and mutates before failure.
  Commodity guards are unsigned; negative deal counts enter a nonzero decrement loop. Only the package
  wrapper's publication of a new object is atomic. Four worked cases distinguish these behaviours.

<!-- from: re-regionalcity (2026-10-20) -->
- ~~**RY2 - §2:** what the record's three unknown strings (`+0xE4`, `+0x10C`, `+0x134`) hold; no getter reads them and
  the samples leave them empty. [as read]~~

  **ANSWERED 2026-10-26 (re-rr29):** MATCHES — [regional-city.md](../../regional-city.md) §2 specifies three
  independent opaque strings, their wire positions, empty construction, persistence and absence of public
  accessors/cache producers. A distinct alpha/beta/gamma preservation case prevents aliasing or dropping them.
  Semantic names require observing an original nonempty producer with field-write stack capture during the
  stated city/locale operations; deserialization and supplied fixture values do not establish a meaning.
