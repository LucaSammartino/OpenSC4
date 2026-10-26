# GZ framework core questions

These questions belong to [gz-framework-core](../gz-framework-core.md). Filed
2026-10-23 by `re-gzservices5`; none is an asserted default behavior.

- **GZF1 — Framework lifecycle and unnamed slots (§2):** Recover exact signatures
  and semantic names for all constructor-installed `0x00AC3090` entries, the
  secondary face, main-loop pacing, ordered callback ties and partial-init unwind.
  The inventory gives stable targets but does not yet define these operations.
- **GZF2 — COM loader and 32-bit IDs (§3):** Settle library enumeration callback
  cancellation, class-index registration/removal, failed-library caching and full
  shutdown ownership. Recover the ID clock wrapper/epoch, bit allocation and wrap
  behavior from instructions before specifying deterministic generated IDs.
- **GZF3 — Command-server edge cases (§4):** Confirm bulk registration/unregistration
  partial failure, null active-dispatcher execution, enumeration cancellation,
  registration notification ordering and generator replacement ownership.
- **GZF4 — Misc command adapters (§5.2):** Recover parameter IDs/types/defaults,
  outputs and failure statuses for the 35 registered commands. Settle wait/cancel
  lifetime, helper duplicate suppression, and whether the repeated popup-safe ID
  and absent popup-enabled subscription in `0x005C82E0` affect actual dispatch.
- **GZF5 — Tool command adapters (§5.3):** Recover the seven parameter schemas,
  coordinate conversion widths/order, drag helper sequencing and failure behavior
  when no view/tool is available. Link tool semantics already specified elsewhere.
- **GZF7 — Logging configuration (§7):** Recover the named level table, numeric
  parsing instructions, Init configuration, duplicate backend-name policy and
  backend ownership. Settle recursive drain behavior, formatting
  overflow/allocation failure and shutdown pending output.
- **GZF8 — File-system/CD policy (§8):** Settle application
  path splitting and string lifetime, INI filename/data-key literals, cache
  invalidation, CD scan order/recursive search, ValidateCDPath and no-disc failure.
- **GZF9 — Script interpreter remainder (§9):** Recover constructor/IID/default
  vocabulary and AddToken overload semantics; numeric textual syntax/overflow,
  unassigned-parameter default variant, final/error callbacks, short reads and
  embedded NUL behavior. Confirm malformed quoted/tagged EOF cases with vectors.

- **GZF13 — Buffer secondary drawing face (2026-10-26, replace-gzfw29):**
  Section 6 and span-render-8 §10.7 establish that QI(`0x86D72B57`) returns
  primary+4, but neither specifies its table beyond QI/AddRef/Release. Please
  specify the complete secondary table, each method's stack arguments, cleanup,
  return channel, and how its color/drawing operations relate to primary slots.
  The replacement currently implements only those three specified secondary
  methods; the 47 primary entries and hardware cache are implemented/tested.
  Key-off control `gzfw29-min-off-r4e` is clean through 2026-04-03, ref_warnings=0;
  key-on `gzfw29-min-on-r4e` faults before the first day, RTC=0/ref_warnings=0,
  jumping to `0x6F6E6160`. Both used DLL SHA256 prefix `240183d7`.
  In the earlier same-DLL `gzfw29-min-on-r4d.log`, lines 1047–1050 record an FC4
  buffer at `0x0522EF60`; fault line 4088 has ECX=`0x0522EF64` (its secondary
  face), EBP=`0x0522EF60`, EDX=`FreeCity4.dll+0x14CF26C`. The first return
  address is `0x008D889F`, inside the ABI-listed `RZDrawBltEdge` range starting
  at `0x008D8800`; no game code was read to derive this report. Final r4e
  reproduces the same invalid target and primary+4 receiver relationship.
  Evidence: main `build-runs/game-results/gzfw29-min-{on,off}-r4e.{log,verdict}`
  and `gzfw29-min-on-r4d.log`. Stop this replacement qualification item pending
  the secondary interface contract; no inferred drawing slots are implemented.
