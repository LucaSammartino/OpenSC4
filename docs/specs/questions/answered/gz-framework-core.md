# Answered questions — GZ framework core

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
