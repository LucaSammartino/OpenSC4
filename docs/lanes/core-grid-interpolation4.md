| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-grid-interpolation4 (two integer byte-grid kernels) | `Codex` / `core-grid-interpolation4` (engine lane) | ready: implementation `07be0ee9`, unchanged engine `32938cf8`; round 2 canonical cargo 5391/0, clippy exit0/0 diagnostics, native exit0, ctest 68/68, qbuild exit0; all 19 grid tests PASS; historical baseline E0432/partial20 retained; no game runs/new questions; ABI unchanged | - |

## Round 2 terminal qualification

Reviewed the published algorithm and GINT1-GINT3 before first execution. All
G1-G12 literals match (13 body cases, including both G10 bodies). Quantized
coordinate 32767 gives middle byte 127; two rounding stages give 1 rather than
0. Signed loads, upward negative ties, exactly one final 128 bias, singleton
axes, reduction and asymmetry remain represented. The three maximum-count
cases independently give 77, [64,64] and [191,191], including unused wrapping
terminal advances. Checked storage errors occur before writes. No production,
test or expected-value change was needed; the test count remains 19.

One serial `qbuild quick` completed on committed engine source: core Rust
3347/0 over 3 summaries, including all 19 grid tests; dependent crates compiled
without execution; native build 4/4; fc4verify PASS (83.9s), qbuild exit 0.
Checker PID34444 creation1792727409.5641444 and fc4verify PID37420
creation1792727409.6007826 were observed with own cwd/argv. Actual exec session
23546 was collected to its terminal exit. Evidence:
`build-runs/grid-r2-quick-result.json`, `r2-snapshot-035059.json` and
`fc4verify-quick-test.log`.

After milestone `a481a75f`, exactly one canonical `qbuild all` completed:
workspace Rust **5391 passed / 0 failed / 0 ignored** over 24 result summaries;
Clippy **exit 0, zero diagnostics** with warnings denied; native build **exit
0**, 2/2 edges (0.6s); CTest **68/68 passed**, 0 failed (76.11s); fc4verify PASS
(77.0s); **qbuild exit 0**. The canonical run also executed all 19 grid tests.
The 5391 total equals the old 5372 plus the 19 newly executed tests; no tests
were added or removed in this round. Own absolute-path `qbuild fmt` exited 0
for both owned new Rust files. No cosmetic or source rerun followed the gate.

Canonical checker PID27356 creation1792727555.4145749, own cwd/argv, output
and engine identity are saved in `build-runs/r2-snapshot-035254.json` and
`grid-r2-canonical-session.json`. The SAME actual exec session **32993** was
collected through terminal exit 0; no cancellation or duplicate quick/all,
standalone cargo or CTest was launched. Later source snapshot, log sizes/hashes,
summed Rust counts and the complete canonical grid-test block are saved in
`build-runs/grid-r2-final-evidence.json`.

Before and after validation, engine Git tree was exactly
`32938cf87952dce5118c967ae7022c7629141c7d`; tracked state was clean and the
three owned source SHA256 values were unchanged:

- module: `1cd03da279f7101ae076ed41a7c3783561ab022deaab1c03f8142d18269b6ffb`;
- tests: `41ae4ad996a2b7307ef8bb9d13153e4d42bb6fbc78c3c655d6e54218bdadb089`;
- additive lib export file: `0b59fa2d79743cb27461a50eeb30915645e511a92f56b556db7ca02b6336bec2`.

Combined sorted owned-content digest is
`2951881e5f605e0504c3bb0177d11b5472687542dec8515eee9bb1c85772675e`.
This round changes only the lane report; final engine equals gated engine.

## Implementation and supported scope

Added `engine/core/src/grid_interpolation.rs`, one additive module export, and
independent integration tests in source commit `07be0ee9`. Its tracked engine
tree is `32938cf87952dce5118c967ae7022c7629141c7d`; this report changes no engine source.

The two entry points are `interpolate_unsigned` (zero-extended u8 loads, zero
bias) and `interpolate_signed` (sign-extended i8 loads, exactly one final 128
bias). Both take caller-allocated independent outer slices and explicit
outer/inner counts. No allocation, resizing or descriptor mutation occurs.
The integer kernel uses signed 32-bit shifted division, repeated quantized
steps, signed outer clamps, raw-unsigned inner comparison, separate inner
rounding, upward ties and explicit wrapping terminal advances. There is no
float, host interpolation output, pixel/palette pipeline or saturation.
Safe Rust borrows enforce disjoint source/destination and mutable destination
elements; no in-place snapshot semantics are invented.

The checked portable extension validates counts in 1..=32767 and accessible
storage for every referenced element before writes. Typed errors distinguish
unsupported counts and short outer/inner storage. Extra backing storage is
allowed and untouched. This does not emulate native empty-input recovery,
fault handling, allocation failure or rejection outside the ordinary domain.

GINT1 caller allocation/failure/unequal target-city guarantees, GINT2 larger
native counts and reachable wrap cases, and GINT3 overlap/malformed descriptors
remain withheld in `docs/specs/questions/grid-interpolation.md`. No new question
was introduced. No MapView consumer/allocation fix, renderer, region, C ABI,
SDK, CMake, configuration or default behavior changed. The attributed 587+593
byte inventory is not executed-original coverage or a whole-owner claim.

## Independent ledger and measured checks

Complete literal G1-G12 expectations were recorded from public spec section 7
before production implementation or execution: 13 body instantiations including
both G10 cases. G3 discriminates 127 versus an exact-half 128; G4 discriminates
two-stage 1 versus single-stage 0. Signed ties/bias, singleton axes, equal copy,
clamps, reduction and nonuniform asymmetry are represented.

Three independently derived maximum-count cases use only one or two output
bytes: exact middle byte 77, unsigned quarter results [64,64], and signed
quarter results [191,191]. Comments derive coordinates and identify unused
terminal additions exceeding i32::MAX. Sources are only 32767 bytes or singleton
elements, not giant 2-D grids. Additional tests check padded independent storage,
descriptor preservation and typed errors before any write. There are 19 test
functions total; **all 19 passed in both round 2 quick and canonical all**.

Own module/test qbuild fmt with absolute paths exited 0 in both rounds. A prior
round 1 relative-path attempt could not find its files; corrections then changed
only the owned new files. Round 2 changed no engine files.
The lib change is a single export following its existing list style. q.py
could not start Bash (E_ACCESSDENIED); it executed no focused tests.

Baseline qbuild all began before implementation. Its Rust log records 5372
passed/0 failed across 23 summaries. I added the expected ledger while baseline
was running; clippy saw it before the module export and failed E0432. This was
a baseline sequencing error, not a final-source verdict, and is preserved.
Round 1 launched no quick or final-source all. Round 2 results above are from
one new serial quick followed by one canonical all; they do not upgrade the
mixed-source baseline.

## Historical baseline failure and partial result

At 2026-10-23T04:33:07+01:00, read-only inspection showed this own process chain:

- PID 21192: `qbuild.py --wt .../core-grid-interpolation4 all`.
- PID 28180, parent 21192: `fc4verify.py --wt ... build ctest`.
- PID 29472, parent 28180: `sccache.EXE --show-stats --stats-format json`.

The native build log reached 217/217 (final executable link), size 19293 bytes,
last write 04:26:05. That progress did not establish a native terminal exit.
The original exec session was 46957; its last round 1 collection was still
running. A later durable CTest log records only **20/68**, not completion.
Original baseline terminal outcome remains **UNKNOWN**.

Copied `evidence-run4/coordinator-six-stats-observer-interruptions.json` records
the root's case-specific interruption of exact readonly statistics PID29472,
creation1792725967.238675, exit15. Parent28180 and daemon36520 were preserved
at that action. Later absence of 21192/28180/29472 and partial CTest supplies
neither a cause for disappearance nor terminal success. Round 2 did not wait
lost session46957 as live. Fresh own snapshot at 03:49 UTC confirmed no own
named process hits and no listener on port4421, permitting one serial new run.
No lane kill, restart, cache/RPC action, external request or bypass occurred.

Before overwriting fixed log paths, round 2 archived **11 logs/proofs** under
`build-runs/r1-fixed-archive/`, with file sizes/SHA256 in `archive-index.json`.
This preserves real Clippy E0432, old Rust5372, native217 and partialCt20,
plus root action and copied process/admission proofs. No old terminal record
was found in the supplied own evidence. No native game/shadow request was made;
executed-original coverage remains zero. The new gate qualifies the portable
implementation against the publication, with GINT1-GINT3 still open.

## Identity and milestones

Detached original base `5e49ebfe` includes re-grid-interpolation4. Copied host
proofs in `build-runs/evidence-run4/` establish role core, dependency on main
and full admission/resume guard exit 0. Current own manifest matches lane,
engine kind, worktree, **round 2**, base `c589d72b` and git-dir (budget150000,
max_calls100). Runtime codexguard remained within budget. No branch/ref/config
switch or agents were used. Commit output included a packed-refs.lock
permission warning, but detached commits succeeded; no Git repair was attempted.
`0917fec6` records identity/scope; `07be0ee9` implementation and ledger;
`c589d72b` incomplete round 1 report; `b22d773f` preserves baseline and literal
review; `a481a75f` records successful quick and stable engine before canonical all.
No new questions, ABI exports, game requests or consumer changes. Ready for the
normal coordinator merge queue; no remaining lane implementation step.
