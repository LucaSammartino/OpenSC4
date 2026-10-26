# sound-system: answered questions

<!-- from: core-sound5 (2026-10-23) -->
- **SS12 — Deferred list duplicates and drain order:** [literal lists]

For §7.1, when the same handler or target is registered repeatedly, does
unregistration remove one matching entry or every matching pointer? For target
changes deferred during enumeration, are adds/removes drained in request order,
or in separate batches like the explicitly specified handler queues? The core
currently retains each registration, removes every matching pointer, and drains
target changes in request order; handlers drain registrations before removals.
For §9's otherwise ordinary relative paths, is choosing the base directory's
slash style and joining with one separator correct? SS11 already reserves exact
separator/UNC/dot behavior; this records the literal simple-path choice used by
the parser continuation, not a claim that Open is settled.

**ANSWERED 2026-10-26 (re-ss29c):** DIFFERS — §7.1: immediate registration
unregisters before reinsertion; unregistration erases the first match per list.
Targets drain removal, fast-add, slow-add batches; handler adds replay LIFO,
then removals FIFO. §9: segments are concatenated literally with backslash
delimiting, without choosing a slash style or inserting a separator. Worked
cases and instruction citations are in those sections.
