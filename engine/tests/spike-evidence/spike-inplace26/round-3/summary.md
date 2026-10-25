# S3 round-3 result

SPIKE VERDICT: INCONCLUSIVE

Exact/confined criterion: **8/10 (80%)**; **273/273** unmutated comparisons exact.
Outside net-neighbourhood signals **0/10**; float-only differences **0/10**;
refusals during installed hooks **0**. KILL threshold (>30%) is not met.
Confinement means the documented watched regions, not whole-process safety.
Native global light and vertex colours were reached naturally; six other
functions needed fixtures. Two installations refused (`unsupported-entry-or-busy`),
with no more precise reason recorded; neither is removed from the denominator.

| Function | Natural calls (two runs) | Natural samples/exact | Fixture exact | Outside | Charged adapter lines |
|---|---:|---:|---:|---|---:|
| language-from-system | 0 | 0/0 | 16 | no signal | 25.2 |
| system-from-language | 0 | 0/0 | 16 | no signal | 25.2 |
| language-alias | 0 | 0/0 | 16 | no signal | 25.2 |
| primary-language | 0 | 0/0 | 16 | no signal | 25.2 |
| default-country | 0 | 0/0 | 16 | no signal | 25.2 |
| system-country | unbound | 0/0 | 0 | unobserved | 25.2 |
| lighting-vertices | 192 | 104/104 | 16 | no signal | 25.2 |
| lighting-global | 41 | 41/41 | 16 | no signal | 25.2 |
| lighting-shadow | 0 | 0/0 | 16 | no signal | 25.2 |
| lighting-night | unbound | 0/0 | 0 | unobserved | 25.2 |

Teeth: **10/10** one-value deliberate errors detected/located in **8** new
functions, with zero pre-existing errors. Scalar return location is window 6,
byte 0; BGRA output window 2, byte 0; vector output window 1, byte 0.

Actual marginal **and cumulative rates are unknown** because none of the eight
qualified bodies has a published byte count. Known-byte lower bounds are
**0 KB/hour marginal** and `1821 / 1000 / cumulative_lane_hours` cumulative.
The verifier computes both against actual commit timestamps. Round 3 starts
2026-10-25T13:36:50Z; cumulative starts 11:20:12Z. Endpoint is the commit named
`Finish timed round-3 S3 measurement`, including final reporting, waits and fixes.
The >=8 KB target and >=11 KB/hour PASS criterion are **not established**.
Completing native ABI/size facts for the larger exclusions would decide the rate;
this limited result does not establish a cheap oracle for all closed functions.

Adapter/facility additions: **252 physical added code/test/comment lines**, no
prior-facility lines charged again. `adapters.csv` allocates all new work equally:
25.2 per selected function, **31.5 per installed/qualified function**. Each has
one entry-shell line; 242 other lines include shared byte replay, fixtures,
native-layout transport, production input import and its layout test. These
allocations are explicit, not a claim of ten independent 25-line adapters.

Gate: cargo **7841/0**, clippy **0 diagnostics**, native **49/49**, CTest **115/115**;
new Rust fmt **2 files/0 differences**. The full gate reported a changing tree;
only evidence/test-data changed after engine-code commit f93c38824. Both game
runs used DLL fingerprint **39944fb6**, verdict clean, ref_warnings=0.
Round-2 Rust is unchanged. No public C ABI exports, defaults or coverage changed.
Recheck: `python verify-observations.py --raw <WT>/build-runs/game-results`.
Raw multi-MB logs stay in game-results; value-only mirrors and a checker are in
build-runs/spike-evidence/round-3. No instruction bytes or return addresses saved.
