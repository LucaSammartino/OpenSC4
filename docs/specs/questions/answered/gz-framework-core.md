# Answered questions — GZ framework core

## GZF10 — Closure details in §§2, 5–8

Filed 2026-10-23 by core-gzframework5: buffer/support QI, color-key defaults,
reference lifetime, framework accessor ownership, logging owner thread,
Unlock mask, ordered-registration direction and directory-setter invalidation.
Fixture assumptions were construction-thread delivery, ascending priority,
all-flags clearing, preserved valid cache, and guarded Shutdown on final release.

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
