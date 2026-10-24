# region-model: answered questions (archive)
<!-- Archive for [region-model.md](../../region-model.md). Lanes do not read this file. Open questions are in ../region-model.md. -->

<!-- from: region-model spec (2026-10-21) -->
- ~~**RM4 - §5.1:** `SC4ReadAppLoadSaveVersion` (`0x005DBF30`) returns 0 or 65 when the package has no version
  record, depending on a second package call (DB segment slot `+0x4C`). What is that call, and is a city file
  without a version record accepted (65) or refused (0)? Do files saved by the retail 1.1.641 game always carry
  the record? [accepted as 65]~~
  **ANSWERED 2026-10-21** in region-model.md §5.1: no version record counts as 65, an unreadable one as 0; all shipped city files carry the record (found while writing regional-city.md §9.1).

<!-- from: core-region10 (2026-10-24); entries imported from that reserved worktree, absent from this lane's base -->
- ~~**RM15 - §3.1:** does RZAtoUint32HexA reject trailing non-hex text or more than eight digits, or consume a prefix? [literal prefix parser: skip blanks and optional 0x, consume up to eight hex digits; zero remains a literal name]~~
  **ANSWERED 2026-10-24 (re-region-rm18):** MATCHES — region-model.md §3.1 and §12.1: optional prefix, at most eight digits, trailing text ignored; zero retains the complete literal name.

<!-- from: core-region10 (2026-10-24) -->
- ~~**RM16 - §8.1:** what portable contract should GetBaseTerrainHeight expose after SetBaseTerrainType stores a value outside 0..1? The spec describes an arbitrary out-of-bounds process-memory read, not a stable value. [Unsupported::BaseTerrainType for this unsettled input; setter still stores any integer]~~
  **ANSWERED 2026-10-24 (re-region-rm18):** MATCHES — region-model.md §8.1 and §12.1: preserve all type bits; reject height lookup outside 0..1 with Unsupported::BaseTerrainType as an explicit portable choice. The original performs an unchecked 32-bit indexed memory read, with no stable height contract.

<!-- from: core-region10 (2026-10-24) -->
- ~~**RM17 - §5.5, §7.1, §7.6:** RemoveCity and failed ReloadCity leave a dangling box, and ResetTutorialCity would then try to update that freed box. What defined failure behavior should an adapter expose? [retain the stale opaque BoxId on failed reload, resolve it to None, skip tutorial-id update, still activate tutorial after successful export]~~
  **ANSWERED 2026-10-24 (re-region-rm18):** MATCHES — region-model.md §§5.5, 7.1, 7.6 and §12.1: invalidate box resolution without clearing the caller's stale ID; never revive aliases on reuse. On successful export with failed reload, skip the ID write, reactivate and return false. This is the adapter contract; the original dereferences freed storage.

<!-- from: core-region10 (2026-10-24) -->
- ~~**RM18 - §7.4:** sanitizing the whole city filename to 256 bytes can remove its extension. Where is the collision suffix inserted if no dot remains? [append ` (n)` at the end when there is no last dot]~~
  **ANSWERED 2026-10-24 (re-region-rm18):** DIFFERS — region-model.md §7.4 and §12.1: use the last dot of the whole resolved path, even a directory dot. With no dot anywhere, a collision requiring insertion raises basic_string out-of-range; expose a filename-position failure, never append a suffix. Reusable or absent base files need no insertion and still work.

<!-- from: core-region10 (2026-10-24), originally filed in questions/region.md; archived here with the requested owner-closure set -->
- ~~**RQ3 - §6 versus region-model.md §6.4, §7.3, §7.6:** the older spec starts folder suffixes at 1, says MoveCity fails only on rename failure and describes ResetTutorialCity's return as whether export succeeded. The detailed model starts suffixes at 2, also fails a post-rename read, and returns true on failed tutorial export. Which is authoritative? [follow the detailed region-model.md pseudocode: suffix 2, read failure false, tutorial export failure true; ignore the regional-city write result as that pseudocode does]~~
  **ANSWERED 2026-10-24 (re-region-rm18):** MATCHES — region-model.md §§6.4, 7.3, 7.6 and §12.1: first suffix 2; MoveCity false on rename or read failure, write/Init results ignored; tutorial reset true on failed export, false on success/no match. Corrected region.md §6 to agree. Failed tutorial reload has the RM17 safety boundary.
