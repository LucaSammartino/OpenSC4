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
- **GZF6 — Remaining buffer interface (§6):** Reconcile every remaining
  `0x00AC1400` slot beyond the span-render-8 bridge with semantic names/signatures;
  specify resize, attached storage, palette, copy/conversion and allocation failure.
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

<!-- from: core-gzframework5 (2026-10-23) -->
- **GZF10 — Closure details in §§2, 5–8:** Please specify buffer QI IIDs,
  initial color-key state/output-write rules and buffer reference lifetime;
  support QI IIDs/final-release behavior; framework accessor reference ownership;
  which thread establishes log delivery ownership; and reconcile Unlock clearing
  all flags (§6 and span-render-8 §10.4) versus clearing only supplied flags
  (span-render-4444 bridge). Is ordered registration ascending? Does a failed
  directory setter invalidate an already valid cache? [Core fixture uses the
  construction thread for log delivery, ascending order for distinct ordered
  participants, all-flags clear per this spec, and preserves prior cache validity
  on setter failure. Buffer color-key/QI and unspecified identity/accessor
  lifetime entries remain Unsupported; support Release uses the existing command
  RefCount and guarded Shutdown. No native defaults are claimed for these gaps.]

<!-- from: core-gzframework5 (2026-10-23) -->
- **GZF11 — Inventory-only entry contracts (§§2–4, 7):** Several named entries
  have targets but no complete behavioral contract: framework stream setters,
  stream operation/SetStream and propagated debug setting; COM SetServiceRunning
  and enumeration; command service QI/base lifetime, formatted execution and
  generator registration replay/return contracts; logging AddOutputStream,
  DumpOutputLevels, Output versus OutputLevel and unnamed accessors. Please give
  signatures, null/error/output-write behavior and ownership, or explicitly mark
  these entries open. [Only described behavior is implemented; inventory-only
  entries have no claimed native implementation. Formatted bytes can already
  pass through execute_text; no native varargs format policy is guessed.]
