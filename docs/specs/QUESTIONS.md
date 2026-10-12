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
