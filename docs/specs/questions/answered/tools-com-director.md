# Answered tools COM director questions

**TD2 - Registered services' remaining dependencies**

The slot bodies, native widths, open/read/seek/close, creation/list behavior and
reference lifetime now have contracts in §5. Remaining precise dependencies:
what are the native semantic names of the two registered
interfaces (IDs `FCEFF549` and `3AFD825F`) and forwarded embedded-file slots?
What does `_ftol2` return/store on NaN, infinity or out-of-range estimates at
`0094AE63`? The open path checks none of these cases. Names must come from native
evidence. Decoder/header contracts remain TD1, not implied by the stream wrapper.
The framework request IDs `(056B906E,656B8EFC)` are now confirmed in §5.2;
header parser `0094A2C1` is now specified separately in §5.6.

**ANSWERED 2026-10-22 (re-tools-director-questions4):** DIFFERS — §5.2 proves exact
numeric identities FCEFF549/3AFD825F and embedded cRZFile endpoints. +24 returns
raw open byte via004770F0; +4C returns logical position or FFFFFFFF via00918345.
No semantic interface names are established or invented. §5.3 proves _ftol2 uses
signed64 FISTP/correction and stores only low32 bits: masked invalid conversion
returns80000000:00000000, storing0; finite conversions outside32-bit range wrap.
Unmasked relevant x87 exceptions do not guarantee normal return/store. TD-V36..38
discriminate these rules. Reachable decoder arithmetic remains TD1.

**TD3 - Exceptions and singleton construction guard**

`0094AC43` sets the static bit before constructor/exit registration. If construction
or AddCls throws, does the compiler unwind path reset the bit, release allocations,
or leave a partially constructed singleton?

**ANSWERED 2026-10-22 (re-tools-director4):** DIFFERS — §2 specifies retry after
exception: getter descriptor `00B03278` has one unwind state, mapping state 0 to
`00A6F206`, which clears bit 0 of `00B6351C`. Constructor descriptor `00B03254`
maps state 0 to `00A6F1F4`, which destroys its completed base. Both have no catch
table, so the exception propagates rather than becoming a false/null return.
Recursive entry without exception still sees the already-set flag and returns
the in-progress static storage. Exit registration's return value is ignored.
