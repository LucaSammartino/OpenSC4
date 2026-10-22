# Tools COM director questions

## TD1 — Reached decoder bodies and attribution boundary

The 70-body / 31,293-byte inventory in tools-com-director.md §1 includes substantial
reached code, especially `00A6A25B` (5716 B), `00A6B918` (3773 B), `00A675E0`
(3405 B), `00407000` (2444 B), `00A69065` (2084 B), and `00A66CAC` (2078 B).
What are the input formats, exact arithmetic/widths, state transitions, buffer bounds,
error/partial-input returns, and ownership of every U row? Which rows belong to a
decoder/library rather than the director class? `00A66A31` has neighbor attribution
only. Native evidence is required; no complete-owner credit until each row has a
contract or an evidence-backed attribution correction approved by the coordinator.

## TD2 — Registered services' full slot contracts

For classes `3CEC2B47` (table `00AD4ED8`) and `9CEC2B58` (table `00AD4EC0`), resolve
all S rows in §1: file/decode setup, open/read/seek/close, list/cache identity,
ownership, failure cleanup, service interface IDs and native argument/return widths.
The registrations and direct factory behavior are established. Do not infer named
tool classes, message behavior, or service-singleton status from attribution alone.

## TD3 — Exceptions and singleton construction guard

`0094AC43` sets the static bit before constructor/exit registration. If construction
or AddCls throws, does the compiler unwind path reset the bit, release allocations,
or leave a partially constructed singleton? Trace handlers `00A6F1FC`/`00A6F214`
and the exit-registration helper rather than assuming retry or exception translation.
