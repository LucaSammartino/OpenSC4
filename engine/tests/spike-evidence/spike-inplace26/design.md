# S3 in-place experiment

Start milestone: the commit introducing this file. End: the measured evidence
commit, including build/queue/game waiting (report-only commits excluded).
Also report elapsed time from the supplied
base commit, conservatively, so setup time is visible.

Select 20 entries from view-utilities.md sections 12-13: its 18 cached-input
entries already bridged to production Rust, excluding the two scalar-return
entries, plus rotation, ground arithmetic, cross and primitive indices (20).
The scalar hooks are excluded because a float32 capture would change the extended
x87 value returned to the game. This controls adapter
cost and separates numeric disagreement from instrumentation failure. Function
body bytes and signatures come only from that spec. The existing Rust adapter
does not establish agreement; only the new observations count.

Entry jumps use a fail-closed general x86 instruction-length decoder and private
trampolines. Runtime instruction storage is transient implementation machinery;
no instructions, return addresses, or arbitrary memory dumps enter evidence.
Unsupported entry encodings are rejected rather than guessed.

Captured windows include the entire 0x470-byte receiver and all described pointer
arguments. Before/after comparisons additionally cover the containing committed
writable private VirtualQuery regions, capped for bounded observer cost. Thread
stack comparison excludes the hook's own active frames. Executable/image memory
is never snapshotted. A region refusal is an unverified sample, never an exact
result. Outside changes prevent replay; the original's state remains installed.
Otherwise restore mutable outputs, run the Rust adapter on the pre-call inputs,
compare, and restore the original result. Read-only receiver/input windows are
never written back; unexpected changes there are compared to the pre-call copy.

Blind spots: writes later undone, unrelated allocations/global/image storage,
other stack frames below the caller, uncommitted memory, and unsampled calls.
Concurrent changes in watched regions can produce false outside-write reports.
This is an allocation-neighbourhood check, not complete write interception or
a proof of confinement across the process. Report that limitation in the verdict.

Natural calls and explicitly constructed city-load fixtures have separate
counters. First 50 natural calls and then one in 20 are sampled. Debug mutations
change one result word and are compared independently from normal replay, so the
game always receives the original result. Float-only means finite result words
within four ULPs, with no integer/boolean/owner/input differences; retain maxima.

PASS requires >=14/20 exact and confined, every deliberate error detected, and
>=11 KB per measured lane-hour. KILL requires >6/20 with outside writes or only
float-rounding differences. Missing calls/bindings are not positive evidence.
