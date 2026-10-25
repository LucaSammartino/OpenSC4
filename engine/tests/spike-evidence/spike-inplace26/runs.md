# Measured runs (source through 33875e28d)

| Label | Final verdict | Reference warnings | Detail |
|---|---|---:|---|
| inplace26-natural-1 | clean | 0 | 600.625 real seconds, 18.00 days/second, date 2055-09-04. 5,324,602 observed calls; 266,768 selected samples; 186,518 comparisons; 146,979 exact sample results; 39,539 other differences; 55 outside-change samples; 80,195 refusals. |
| inplace26-fixture-1 | corrupt | 0 | One foreign access violation, no completed city/date or final counters. 11 logged deliberate errors across 10 distinct functions were all detected and located before abort. |

The natural request incorrectly put end_date in [ini]; the game logged it as an
unknown key and ran for the 600-second wall timeout. This completed run is kept
as measured, not relabelled a date-bounded run. The fixture request moved end_date
to the request header and used seconds=180. The linked DLL fingerprints recorded
by the harness were 3cf78cdc and d787d163 respectively (a repeat native gate
relinked the second DLL); production source was unchanged between those runs.

The fixture's access violation was in the loaded image at RVA 0x003f785d,
inside ProjectPointToViewportCoordinates (entry RVA 0x003f7850), while attempting
the first constructed call to that entry. Code=0xc0000005. No instruction bytes,
return addresses, raw stack/register dump or unnormalised image pointers are
copied here. Cause is unresolved: native machine-state preservation, an entry
binding/relocation problem, and observer/mutation interference need isolation.
Do not interpret the clean harness verdict of the natural run as a proof that
the observer preserved the native result semantics.

natural-1.csv contains final natural counters; functions.csv supplies body sizes,
signatures and adapter lines. natural-1-outside.csv records the reported region
and offsets; writes have unknown attribution. Region 0 in these calls contains
the receiver. The whole containing region includes unrelated allocation data;
the observer cannot identify which allocator object or thread changed it.

fixture-1-mutations.csv contains every logged mutation: 11/11 detected, 10 unique
functions (cross, zoom-rotation, view-point, clip2, view-viewport, fixed, view-box,
city-corner, clip3, world-view). Window 99 is the boolean return; other windows
are indexed in the adapter's Capture list. All mutations are at word zero and
xor one bit. The normal replay comparator supplies the location/detection.
These events establish only completed-sample lower bounds for the aborted
fixture. No final per-function fixture counts exist; none are invented.

Both runs failed to install rotation and visible-edges (reported reason is
unsupported-entry-or-busy, not a proven absence of ABI). Their denominator stays
in the selected 20. Natural view-viewport, world-viewport and viewport-world had
zero calls. clip3's three natural calls were refused for receiver/output alias.
All other refusals were not-private-writable-region. The mismatching functions
were view-box (boolean), box-rectangle (integer pixels), world-viewport2 and
viewport-fixed (large float differences). There were zero four-ULP float-only
differences. These are unresolved mismatches, not rounding-only verification.

Strict provisional exact/confined natural cohort: clip2, city-corner, cross,
ground, primitive-indices, view-point, view-world = 7/20 (35%), 1031 body bytes.
Ten functions had only exact compared samples if outside changes are ignored.
Six functions had outside changes = 30%, not the >30% KILL threshold. Because
the observer later faulted, no body bytes are asserted independently verified.

SPIKE VERDICT: INCONCLUSIVE. PASS exactness fails (7 < 14); observed KILL fraction
is 6/20, float-only 0/20; teeth are positive for 10 functions but the full cohort
is incomplete. Rate is computed from lane commit times in metrics.md, including
build and game waits. A transparent native register/FPU-state test, isolated
per-entry replays, and clean completed counters for all 20 would decide the next
attempt. All switches remain off by default; keep this branch WIP.
