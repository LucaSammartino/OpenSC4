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
