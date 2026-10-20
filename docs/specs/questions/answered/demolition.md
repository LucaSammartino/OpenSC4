# demolition: answered questions (archive)
<!-- Archive for [demolition.md](../../demolition.md). Lanes do not read this file. Open questions are in ../demolition.md. -->

## From the city situation, demolition, dispatch, port, prop developer and city detail specs (re-specs10 lane, 2026-10-14)

Open points from city-situation.md, demolition.md, dispatch.md, port-developers.md, prop-developer.md and
city-detail.md. The current answer is in brackets; each entry says what would settle it. "Turbo run" means the
in-game harness with the FreeCity4 turbo clock and a message log.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM1:** demolition filter step 7: in a view level other than 1, networks with any flag of `0x1F4F` are refused.
  Is that mask the surface networks, so that only pipes and subways can be demolished underground? [yes, from the bit
  layout. In a turbo run, demolish a rectangle over a road crossing a pipe in the underground view: the road must
  stay and the pipe go]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** in part (demolition.md §4.2 step 7). `0x1F4F` is every network type bit except 4 (water pipe), 5 (power line) and 7 (subway), so in a view level other than 1 a network passes only when it carries none of the other nine types: pipes, subways and also type-5 power line pieces. Power poles are a separate occupant type and step 7 does not test them. The bracket's "pipes and subways" is right for the underground networks.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM2:** `ModifyTerrainHeight` (slot 9) returns a vertex count and never writes `*cost`. [as specified; no shipped
  caller passes a cost pointer. A call trace of slot 9 with the level and raise tools would confirm both]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES, now verified (demolition.md §4.7). `0x00467BB0`–`0x00468191` (`ret 0x28`) never loads its tenth argument; the ninth (`reason`) is read for the redisplays (`0x00467F59`, `0x00468157`). It returns the vertex count.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM3:** `0xA9DED179` has a handler and no poster. [no poster in the exe; FreeCity4 keeps the handler. A turbo
  run with demolitions, disasters and terrain edits should log 0 posts]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (demolition.md §3). The ID's bytes occur only at `0x00466A03` (Shutdown), `0x00466F29` (Init) and `0x00467002` (DoMessage) in the whole file.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM4:** occupant flag 1 means "inserted in the occupant manager", so poles removed earlier in the same call are
  skipped. [yes, inferred from the occupant manager's insert and remove paths; flora.md agrees. A breakpoint on the
  occupant manager's insert and remove watching flag 1 would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES, now verified (demolition.md §4.6). `InsertOccupant` returns early when flag 1 is set (`0x0049EF8E`) and sets it before posting `0x99EF1142` (`0x0049F202`); `RemoveOccupant` needs it set (`0x0049F264`) and clears it after posting `0x99EF1143` (`0x0049F382`).

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM5:** (cores12a) is an occupant of foundation type that lacks `cISC4FoundationOccupant` rejected by the filter
  (step 5)? [rejected]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (choice; demolition.md §4.2 step 5). The original does not test the QueryInterface result (`0x00466817`–`0x00466835`) and calls slot 9 through a null pointer, so it crashes. Every shipped foundation has the interface. Rejecting is the safe *[choice]*.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM6:** (cores12a) should the 0.05 level tolerance (§4.7) be compared after widening `float` to `double`?
  [widened]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** (demolition.md §4.7). Both modes compute `|altitude − h|` and compare it with the `float` constant 0.05f (`0x00A880D8` = `0x3D4CCCCD`): area mode counts `> 0.05f` (`0x0046802A`–`0x0046803E`), per-vertex mode skips `< 0.05f` (`0x00467D45`–`0x00467D59`). At 24-bit x87 precision the subtraction rounds to `float`. Use `f32` throughout. A `f64` compare with 0.05 disagrees within one `float` step of the threshold.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM7:** (cores12a) in area mode, is the redisplay rectangle the same as the `SetAltitude` rectangle? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (demolition.md §4.7). `SetAltitude` (terrain slot 49, `0x00468172`) and `RedisplayTerrain` (slot 68, `0x00468187`) get the same local rectangle `(max(x0,0), max(z0,0), min(x1,cellsX), min(z1,cellsZ))`. Area mode returns `n` also when the area is locked; an immovable occupant gives 0.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM8:** (cores12a) group effect: are the size thresholds strict or inclusive, and is the centre cell truncated or
  rounded? [the literal reading of §4.8]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (demolition.md §4.8). The thresholds are strict and unsigned: `+2` if `n > medLarge` (`0x00B07F4C`, `jbe` at `0x004675BB`), else `+1` if `n > smallMed` (`0x00B07F48`). The occupant effect's model thresholds are strict too (`0x0046740E`, `0x0046743C`). The centre is `(x0 + x1) / 2`, `(z0 + z1) / 2` in signed division truncating toward zero (`0x00467555`, `0x00467568`), the same as Rust's `/`.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM9:** (cores12a) which network kinds does `UpdateNetworks` queue, by occupant type? [the literal §4.6 list]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (demolition.md §4.6), provided the test is the occupant type alone. Type `0x2890D4DE` (pole) is sent `0xAA7A014D` and removed. Every occupant of type `0xC772BF98` is queued (AddRef, then neighbours slot 8), whatever its network kind: there is no QueryInterface and no network-type filter. Any other type is skipped. Flag 1 (slot 32) gates both. The list goes to the network manager's slot 6 (`0x00468396`).

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM10:** (cores12a) is the deferred-lot message's lot released even when the lot is not initialized? [always
  released when non-null]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** in the letter; the choice is accepted (demolition.md §3). The original releases the lot only on the initialized path (`0x004670A0`). A null or uninitialized lot jumps to freeing the record (`0x004670A7`), and its reference leaks *[quirk]*. Nothing posts `0xA9DED179`, so releasing on both paths cannot be observed.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM11:** (cores12a) pole orphans: the core neither holds nor releases references on them. Is that right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES in net effect (demolition.md §4.3, §4.4 step 8). The original holds references: the remembered neighbour poles sit in a vector of auto references, released at the end. Each orphan's QueryInterface reference goes into `orphans` without a further AddRef, or is released when `orphans` is null (`0x0046937B`). Step 8 pops each orphan, inserts it into `occs` and `demolished` (each set takes its own reference), and releases the list's reference (`0x00469031`). The balance is zero. The glue must keep the sets' own references.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM12:** (cores12a) is the view filter still applied when flag `0x2000` (lots only) is set? [skipped for
  lots-only calls; the bare filter is used for the lot-blocking test]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** in part (demolition.md §4.4 step 2, §4.5, §4.9). Two parts match: with `0x2000` (or privilege 0), Φ has no view filter, and the lot-blocking test always runs Φ with the view filter cleared. The original still fetches the 3D view's filter (`0x00468473`) and uses it directly in two tests. One is the empty-lot qualification, states 1 and 6 (`0x00468960`–`0x00468974`, view filter slot 4 with `0x278128A0`). The other is the step-5 effect gate (`0x00468D02`–`0x00468D12`, slot 3). §4.5's "no view filter is used" was wrong and is corrected.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM13:** (cores12a) occupant types not listed for effects get effect nibble 0. Is that right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (demolition.md §4.8). The nibble helper (`0x00466A90`) leaves 0 for foundation, construction and other types, and returns true, so the effect still goes ahead with key `cause << 8 | size`; the lookup falls back to the key with low nibble 0. Only a prop whose `0x8A5E5DB8` is missing or false returns false (no effect).

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DM14:** (cores12a, "DM choice") a `DemolishLots` list that holds only null entries is treated as empty (returns
  true, cost untouched). Acceptable? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (choice; demolition.md §4.5). The original calls lot slot 14 on the first entry (`0x004670F9`) and on every entry, with no null test, so any null entry crashes it and there is no original result to match. Treating it as the empty list is the natural choice; with mixed entries the union covers only the non-null ones. Note: a non-empty list whose lots are all non-demolishable is different. It still calls `DemolishRegion` with an empty mask, which writes `*cost := 0` and, when `doIt`, runs `QueueRemovals(true/false)`.

<!-- from: From the enable=/spec-contradiction wave (filed by the coordinator from the implementer lanes' reports, 2026-10-16; answered by re-answers31) -->
- ~~**DQ2** (`demolition.md` §2): the cell counts are called "city slots 93, 94" but
  `docs/abi/interfaces.tsv` names those `GetLightingManager` / `GetDrawContext`. `CellCountX`/`CellCountZ`
  returned 128x128 in game. Which is right?~~ **ANSWERED 2026-10-16 (re-answers31): MATCHES the spec.** Slots
  93 and 94 really are the cell counts (`0x007C1910` `mov eax,[ecx+0x118]`, `0x004E82A0`
  `mov eax,[ecx+0x11c]`); the `interfaces.tsv` names are code-fold misattributions. `cISC4City.h:218-219`
  declares `CellCountX`/`CellCountZ` there, and `GetCitySizeType` (slot 87) discriminates `+0x118` against
  `0x40`/`0x80`. 128x128 is correct. Held ABI rows in `build-runs/pending-abi/re-answers31-rows.tsv`. Written
  into [demolition.md](../../demolition.md) §2.

<!-- from: From the enable=/spec-contradiction wave (filed by the coordinator from the implementer lanes' reports, 2026-10-16; answered by re-answers31) -->
- ~~**DQ4** (`demolition.md` §4.4 step 4): `lotManager.GetLot(x, z)` is called "slot 5"; slot 5 is
  `GetLotAtPoint` and slot 6 is `GetLot(x, z, bool)`, which every glue lane calls. Which?~~ **ANSWERED
  2026-10-16 (re-answers31): DIFFERS.** The cell overload is **slot 6** (`0x0072B410`, pop 12,
  `GetLot(int32, int32, bool)`); slot 5 (`0x0072A1C0`, pop 4) is `GetLot(cS3DVector3 const&)`. The SDK
  `cISC4LotManager.h:42-43` has them the other way round and is wrong, as `crime.md` already records. Written
  into [demolition.md](../../demolition.md) §4.4.
