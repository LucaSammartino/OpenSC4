# Terrain ripple portable conformance

Current per-section owner closure and §6 vector mapping are in
[README.md](README.md). This follow-up implements settled WR1 and WR11–WR14,
plus WR8's clarified receiver/lifecycle contracts. The focused run passed 72
terrain-ripples tests before adding the reduction-order cancellation fixture
and strengthening the backend-through-owner assertion. Final gate counts are in
[the lane report](../../../../../docs/lanes/core-ripples5b.md).

Owner fixtures now cover ordered trace, partial pairs, UV/size splitting,
configured fade, bounds, contact removal, foreign drawing and native sampling.
The historical WR8-only execution below is retained as earlier evidence, not
the result of this revision. Vector 21 now also has owner texture-failure/retry
assertions in `resource_failure_retry_preserves_old_strip_and_missing_rng_is_explicit`.

Source: public water-ripples §§2.4,2.6,2.7,6. These are independent literal fixtures;
no original/native output oracle or runtime ownership credit is claimed.
The original 37 portable tests and historical 4314 Rust / 67 CTest record are retained
as history, not a result for this revision.

| Public vector | Owned operation / literal discriminators | Status |
|---|---|---|
| 19 | logical zero refcount, nondeleting RemoveRef, Release deletion decision, QI output preservation | PORTABLE EXECUTED |
| 20 | shared process reset, retained arrays transformed a second time, initialized guard before flag test | PORTABLE EXECUTED |
| 21 | missing whole exemplar continues Init; present missing/empty spacing typed Unsupported, no rollback fiction | PORTABLE EXECUTED (spacing/lifecycle only; texture-failure path excluded) |
| 22 | .75 bool false; -1.75 u32 FFFFFFFF; Uint64 high-half bool vs low-half u32; all numeric tags; temporary destruction on helper false | PORTABLE EXECUTED |
| 23 | count six copied, sixth untouched, immutable source, absent minimum clamp, exact 00..0D order, repeated absent 02/0B transforms | PORTABLE EXECUTED |
| 24 | separate nested identities; edits/source destruction; prefix-node reuse; self-assignment; retained capacity; grow allocation before old free; forward rectangle/vertex/node free order; fixed-position insertion and erase output | PORTABLE EXECUTED |
| 26 | elapsed microseconds 175 vs 175000; one advance and discarded overshoot | PORTABLE EXECUTED |
| 27 | stored factor 3C8EFA35, 60 degrees → 3F860A92, retained absent-array retransform vs omitted whole exemplar | PORTABLE EXECUTED |
| 28 | host startup XOR, ordinary zero, remembered/third words, sentinel time/clock fallback, shared stream across owners/reset/Shutdown/re-Init | PORTABLE EXECUTED |
| 29 | delayed equality/no detection, > threshold single detection/retained clock/new-strip elapsed, immediate pending double detection, zoom pending single detection | PORTABLE EXECUTED |
| 30 | named spline spills, asymmetric reductions, retained Tz, axis and non-axis vertices, unclamped dot/library boundary | PORTABLE ONLY; not native x87 bit equality |
| 31 | fresh/repeated/stopped owner lifecycle, no destructor Shutdown, remove-last frees only final list payload | PORTABLE EXECUTED |

Execution: `fc4verify-quick-test.log` reports 3416+21+13+19+0 = 3469 passed,
0 failed; 51 terrain-ripples tests (37 retained, 14 new WR8). Quick remains
without terminal completion in the post-build sccache stats client; this is
test execution evidence, not a complete gate. Clippy, canonical all and CTest
are still unmeasured. Detailed process proof is in this lane's build-runs.

Retained tuning replaces the old all-properties/fixed-five/provider-decoded read.
The old fixed-five `RippleTuning` remains explicit input to isolated geometry kernels,
not process globals or a fallback exemplar. `WaterRipple` uses shared mutex-protected
process storage, including getter snapshots; no RefCell or per-instance read flag.
Shutdown retains first-update/zoom/rebuild state. Rust Drop safely frees Rust resources;
only explicit logical reference/storage events describe native chronology.

Remaining domains: WR2 flag meaning; WR3 selection-byte
meaning; WR8 unsafe backing,
foreign conversion/allocator/error/reentry, full catalogue 69 vs 65 and Pick argument
semantics. The stored WR11 degree factor is fixed in code. Period-bound
arithmetic is supported for period <=2^24 with exact product/floor; larger values
and unproved inherited numeric modes remain Unsupported. Outer-vector insertion,
allocation failure and general overlap/self-insertion are not added contracts.
WR12 startup takes host primitives; WR14 clears pending before delayed detection
but preserves immediate-message double detection. WR13's ledger and spills are
implemented through primitive arithmetic, including backend use by the owner.
The default backend uses binary64/host math and does not claim x87 equivalence;
an exact backend needs matching caller control word and native transcendental/
exception semantics (§1.3 compat-limit). No C ABI/plugin/map/game/native
verification changes.
