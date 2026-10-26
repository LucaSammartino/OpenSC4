# Answered questions — GZ framework core

## GZF6 — Remaining buffer interface

<!-- from: re-gzservices5 (2026-10-23), replacement follow-up replace-gzfw29 -->
- **GZF6 — Remaining buffer interface (§6):** Reconcile every remaining
  `0x00AC1400` slot beyond the span-render-8 bridge with semantic names/signatures;
  specify resize, attached storage, palette, copy/conversion and allocation failure.

**ANSWERED 2026-10-26 (re-gzf29):** DIFFERS —
[§§6.1–6.4](../../gz-framework-core.md#61-complete-primary-table-inventory-gzf6)
reconcile all 47 entries and their return channels. There is no preserving resize,
foreign-storage attachment or palette operation: disposal/reinitialization owns
replacement heap pixels and types 1/3 resolve from the display format. Null pixel
allocation still returns successful Init. Packed pixel access, color/key rules,
clipped fill, line quirks and all type-4–9 copy conversions are specified, including
no-write successes and overlap. §§6.5–6.6 give creation/fill/cache/release and
failure/conversion vectors. The previous type-3 palette interpretation is corrected.

## GZF12 — Buffer hardware-cache contract blocks replacement

<!-- from: replace-gzfw29 (2026-10-26) -->
- **GZF12 — Buffer hardware-cache contract blocks replacement (2026-10-26,
  replace-gzfw29; GZF6 follow-up):** Please specify native slots 37/38
  (`GetHardwareCache` / `SetHardwareCache` in the SDK): cache interface identity,
  getter ownership, setter retention/replacement/null behavior, return value,
  and storage/modification invalidation obligations. The bounded bridge in §6
  and span-render-8 §10.7 does not cover these calls. With the new default-off
  `gzfw` factory enabled, conservative refusal reaches these slots repeatedly
  before city load and produces a broken region UI and `no-load` after 900 s.
  Evidence: main `build-runs/game-results/gzfw29-s0-on-r1.log` / `.verdict` and
  `build-runs/q-main-gzfw29-s0-on-r1-end.png` (DLL f966786a). The same gen27 s0-grow
  script with the key off executes all 216 actions and has 25/25 exact owner
  shadows; its only exception is the already-known native construction Shutdown
  fault. This does not isolate the cache as the sole cause: other GZF6 buffer
  operations are also conservatively refused. Replacement qualification stops
  here rather than inventing cache ownership or claiming that no-load is a
  city-level comparison. Full buffer semantics remain necessary for credit.

**ANSWERED 2026-10-26 (re-gzf29):** DIFFERS —
[§6.5](../../gz-framework-core.md#65-hardware-cache-attachment-and-lifetime-gzf12)
specifies the borrowed getter, primary cache IID `0xFA59F7B9`, ordered release /
Init / retention, false-Init detachment and null-input fault. SimGL keeps a
borrowed buffer pointer; modifications are detected through slot 36, with explicit
Invalidate only for color-key changes. Worked attachment, refresh, replacement,
failure and release cases provide callback/refcount expectations. This is static
original-program evidence, not a new game-run or replacement-qualification claim.

## GZF10 — Closure details in §§2, 5–8

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

**ANSWERED 2026-10-23 (re-gzframework-gzf5):** DIFFERS —
[§2.2](../../gz-framework-core.md#22-service-and-participant-registration) orders
signed priorities descending with insertion-stable ties; §2.4 accessors are
borrowed, and slot 22 is a byte flag, not a framework pointer. §5.1 support QI
accepts only `0x452294AA`; final Release deletes without Shutdown. §6 buffer
QI accepts `1`, `0x20732180`, `0x86D72B57`; fresh color key is disabled and its
getter leaves output untouched; count-one Release deletes, count-zero Release
underflows. Unlock clears only supplied bits. §7.2 binds logging ownership in
Init. §8.2 MATCHES preservation of prior cache validity on setter failure, but
the newly stored nonexistent path then remains the cached getter result.
Conformance vectors GZC39–GZC52 cover these contracts; span-render-8 §10.4 wording
is corrected to agree with the instructions and span-render-4444.

## GZF11 — Inventory-only entry contracts (§§2–4, 7)

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

**ANSWERED 2026-10-23 (re-gzframework-gzf5):** DIFFERS —
§2.5 specifies borrowed lazy stream getters, the guarded QI bridge, destructive
SetStream replacement and separate basic/propagated debug values. §3.3 specifies
byte-store running state, cancellable borrowed callback enumeration and counted
AddRef'd array enumeration. §4.3 supplies all command QI identities, delete-on-final
Release, bounded formatted execution (including first-vararg forwarding), and
idempotent generator registration/replay/removal with ignored callback results.
§7.3 specifies stream-wrapper success even on registration rejection, borrowed
name sets, void diagnostics, guarded printf Output/OutputLevel with different
level selection and unchecked overflow/allocation behavior. GZC53–GZC74 cover
these contracts. Remaining GZF1–GZF9 work is explicitly retained; none of those
broader owner closures is claimed by these answers.

## GZF13 — Buffer secondary drawing face

<!-- from: replace-gzfw29 (2026-10-26) -->
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

**ANSWERED 2026-10-26 (re-gzf30):** DIFFERS —
[§6.7](../../gz-framework-core.md#67-secondary-drawing-color-face-gzf13)
specifies the complete five-entry secondary table `0x00AC13EC`. In addition to
the three receiver-adjusting lifetime entries, slot 3 sets four low-byte RGBA
components and slot 4 gets four byte outputs, each with 16-byte callee cleanup
and an AL boolean result. These are independent drawing-color metadata, not
primary pixel map/unmap methods. All-255 RGBA is the absent-color sentinel;
the setter still succeeds, while the getter fails without output writes.
`RZDrawBltEdge` calls this getter at `0x008D889C`, returning to `0x008D889F`.
GZB9 and GZC75–GZC80 give implementation-facing dispatch, sentinel, byte-width,
ownership, pixel-independence and aliasing cases. The requested contract is
settled statically; no runtime observation is needed to define these entries.
Replacement game qualification remains with `replace-gzfw29`.
