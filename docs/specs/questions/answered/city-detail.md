# city-detail: answered questions (archive)
<!-- Archive for [city-detail.md](../../city-detail.md). Lanes do not read this file. Open questions are in ../city-detail.md. -->

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**CD1:** which lot configuration objects are dropped at each city detail level? [the lot configuration manager
  filters objects by the detail preference when it builds configurations (it passes the preference to the
  configuration's reader); the exact field belongs to a lot configuration spec. Comparing one lot configuration's
  object count at detail 1 and 3 would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** Answered (city-detail.md §3, "Which objects a detail level keeps"). `ConvertPropertiesToObjectArray` (`0x00479D10`) keeps an object when its type byte is 0 (building) or `(value1 & 0x30) < 16 × d` (`0x00479EB1`–`0x00479EC6`): level bits `0x00` at every detail, `0x10` from medium, `0x20` only at high, `0x30` never. Network lot configurations use d = 3 (`0x0060F80E`). MATCHES the bracket; the field is value 1's bits 4–5. Confirmed independently by re-specs13d (prop-lotconfig.md §B3.3, with the shipped counts: 73,832 / 39,781 /
  23,265 objects kept at detail 3 / 2 / 1, and the rebuild at the next city `Init` after the preference changes).

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**CD2:** after a detail change every network lot is redeveloped (network lot manager slot 12). Does that change
  saved state, such as street props drawn from the network lot manager's generator? [yes, inferred: redevelopment
  re-creates the network lot's objects. Saving a city, reopening it at another detail level and saving again, then
  comparing the street prop records, would settle it]~~ **ANSWERED 2026-10-15 (re-specs13c):** the bracket
  MATCHES. `RedevelopLot` (`0x0060D5E0`) removes the cell's street props (prop manager slot 33) and flora (flora
  slot 20), then `CreateLotObjects` draws again from generator `0x00B44870` for every prop and flora object with
  candidates, so the prop manager's and the flora's saved state change; the network lot record (component 11)
  does not. network-manager.md §8.6 and §17.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**CD3:** Init compares the saved level with the current preference on every call, not only the first; a second Init
  at the same level does nothing. Confirm.~~ **ANSWERED 2026-10-16 (re-answers22):** MATCHES (city-detail.md §3.1). `Init` (`0x0045DD10`) has no initialised flag: every call compares the stored level with the preference, runs reduce or increase, and stores the preference; a second call at the same level does nothing.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**CD4:** increase step 4 is implemented literally: one creation per required object missing from the lot; optional
  objects are never created. Is an object present by its tag alone, whatever the occupant's type? [by tag]~~ **ANSWERED 2026-10-16 (re-answers22):** **DIFFERS** in part (city-detail.md §3.3). The presence test (callback `0x0045D5E0`) counts only occupants whose type is flora `0x74758926` or prop `0xA823821E`, then their lot tag; any other occupant type is ignored whatever its tag. Within those two types the tag alone decides (a prop may satisfy a flora object and the reverse). Step 4 is literal (one creation per missing required prop or flora object, optional never created), but it walks every object: an object of another type whose ID is in `need` still costs a draw when its ID list is not empty and creates nothing (`0x0045DC3F`, `0x0045DC4F`–`0x0045DC6F`).

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**CD5:** the increase pass's session generator counter n (derive_seed(S, 0xEAD4BA36 XOR n)) is kept per thread and
  per session seed. Should it reset on a new city load within the same session? [it does not reset]~~ **ANSWERED 2026-10-16 (re-answers22):** **DIFFERS** (city-detail.md §3.5, foundations §6.3). The original seeds a fresh clock temporary each pass and keeps no counter; foundations §6.3 counts clock-seeded temporaries per owning object, from 0 at its construction. The city makes a new city detail manager for each city, so n restarts at 0 at every city load (and is 0 in practice, since `Init` runs once per city). A per-thread, per-session counter that does not reset gives a different seed on the second city of a session.
