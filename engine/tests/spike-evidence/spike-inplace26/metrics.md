# S3 criteria

SPIKE VERDICT: INCONCLUSIVE

Lane interval is first lane commit 6595b1ae5 (11:20:12 UTC) to measured-evidence
commit b7a878282 (12:11:16 UTC), 3064 s = 0.851111 h. It includes gate/build/queue
and game waits. From the supplied base commit, 3432 s = 0.953333 h. Final metric
and lane-report commits are excluded. metrics.json retains full IDs/times;
`python engine/tests/spike-evidence/spike-inplace26/metrics.py` rechecks the rates.
KB means 1000 bytes (all-body upper bound also only 3.711 KiB/h).

| Criterion | Measurement | Outcome |
|---|---|---|
| At least 14/20 exact with confined writes | 7/20 provisional in the natural run, 35%, 1031 body bytes; later fixture fault prevents a verified claim | Not met |
| Deliberate error in at least five functions; every error detected/located | 11/11 recorded error events detected at word 0 in 10 distinct functions; aborted fixture leaves other functions untested | Positive teeth, incomplete cohort |
| At least 11 KB verified per lane-hour | 0 independently verified bytes = 0 KB/h; provisional exact/confined observations would be 1.211 KB/h; even all 3234 bytes give only 3.800 KB/h | Not met |
| More than 30% outside writes or float-only differences -> KILL | Outside-change signals in 6/20 = 30%, writer unknown; float-only 0/20 | Threshold not met |

The 1.1 KB/h run-5 baseline is supplied by the brief, not independently remeasured.
The provisional rate is not verification credit. Full-body byte accounting does
not imply every branch/input was exercised. This single-family cohort does not
establish rates for the other candidate owners. No missing Rust algorithm needed
implementation; algorithm implementation time was zero, not the total lane time.

To decide another attempt: demonstrate full native machine-state transparency on
controlled entry stubs before touching the game, isolate the faulting composite
entry, distinguish observer effects from adapter differences, and repeat clean
natural/fixture runs with final per-function counters and confined-write evidence.
