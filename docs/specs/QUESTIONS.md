# Open spec questions

Questions from engine authors that the spec writers need to answer from the original. Answered questions move into
the relevant spec, and their entry here is deleted.

## Growth developer (growth-developer.md)
- GD1. §14 test 11 gives f = 0x3E4CCCCD and a phantom term of 40, but by §5.3's rule r24(20 × 0.01f) is exactly halfway
  between 0x3E4CCCCC and 0x3E4CCCCD and rounds to even, 0x3E4CCCCC (as protection test 25 has for 20). Then
  r24(200 × f) = 39.999996 and the term is ftol = 39. The core and gd14_11 follow the rule (39); please correct the test.
- GD2. §5.5: is supply(0x3301, 0x10000) queried when n2 = 0, and are the city-size queries made when there are no
  growth items? The core always queries (reading "query the neighbours and both C nodes" literally).
- GD3. §5.4 step 3: which algorithm sorts the decline list? The core assumes the same std::sort as the growth list.
- GD4. §4.3: in which order does the float accumulator visit the covered source tracts when W is coarser? The core
  assumes x outer, z inner, like the integer walker.
- GD5. The StateChanged (0xE8BDA2E7) data4: lot-developer.md §3.2 calls it the SetState "reason", while demand.md §7,
  growth-developer.md §6.3 and building-development.md §3.2 call it the occupant wealth (ReoccupyBuilding passes the
  wealth as SetState's second argument). Which cISC4Lot::SetState argument becomes data4?

## Building development (building-development.md)
- BD1. §4.9 PlopBuilding: "fourth-from-last field" is ambiguous. The core reads `land_value_max` as the orientation.
- BD2. §4.13: when `0x47E2C380` has fewer than 8 bytes the original reads past the data. What should a replacement
  do? (The core keeps the default for a word without 4 bytes behind it.)
- BD3. §4.7.1: the 31-entry building category table at `0x00AB0A40` is not documented.
- BD4. §4.7.2: do `IsBuildingTypeValid` and the type enumerations use the key map or the profile map? They differ
  after `GetBuildingProfile(unknown)` inserts a zero profile. The core uses the key map.
- BD5. §4.7.1: the STLport `hash_map` growth policy is inferred (193 buckets, resize to 389 before the insert that
  would overflow, insertion at the bucket head, head-first rehash). Please confirm; it fixes the enumeration order.
- BD6. §4.4 / §6: does the marks lookup (`operator[]`) create an empty entry, so a developer with no marks is saved
  with a 0-cell set? The core assumes yes.
- BD7. §4.11: is `DemolishLots` called even when nothing is queued? The core calls it unconditionally.
- BD8. §4.9: who owns the reference when `AddNewBuildingOccupant`'s insert fails? lot-developer.md settles only the
  success case (the caller borrows). The core releases its creation reference and returns null.
- BD9. §6: on load, the `operator[]` overwrite of an existing developer entry seems to leak the old reference. Confirm.
- BD10. §4.7.4: an ID in a lot configuration's object list that is neither a family nor a building type adds nothing
  (inferred). Confirm.
- BD11. §1.4: does `Shutdown` before `Init` do nothing? The core assumes so.
- BD12. §4.2 says `IsMapUpdateFinished` is where the growth developers do their incremental work, while
  growth-developer.md §3.1/§3.2 has the work in the developer's own `DoIdle` (its own idle and day agents) and slot 28
  as a pure `state = 0` query. The cores follow growth-developer.md; please align §4.2.
- BD13. §4.11 passes the 32-bit terrain-change kind to `UpdateLotElements(lot, bool hadFoundation)` (lot-developer.md,
  slot 37). The core passes `kind & 0xFF ≠ 0` (the callee reads the low byte). Confirm.
- BD14. §8 gives `GetLotsInCellRect` an AL return; zones.md has an EAX count. (No effect on the BDS.)

## Tract developer (tract-developer.md)
- TD1. §4.4: how does a lot's zone "convert" to the developer's target zone (slot 5)? The core assumes
  1–3 → 1, 4–6 → 2, 7–9 → 3.
- TD2. §4.4: where does a takeover candidate's anchor cell come from? The core uses the lot's location.
- TD3. §4.16 with §4.10.2: a subdivision candidate never sets its rectangle, so the anchor test receives
  (−1, −1, −1, −1). Is that the original's behaviour?
- TD4. §4.13: the cell order inside `IsAggregationLegal` is not given. The core walks rows (z outer, x inner).
- TD5. §4.1: do the inlined copies of the suitability checks count in the histogram?
