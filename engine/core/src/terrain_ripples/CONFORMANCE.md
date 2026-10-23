# WR8 bounded portable conformance

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

Remaining domains: WR1 raw time/controlword; WR2 flag meaning; WR3 selection-byte
meaning; WR4 ordered shore trace; WR5 complete spline/strip walk; WR8 unsafe backing,
foreign conversion/allocator/error/reentry, full catalogue 69 vs 65 and Pick argument
semantics. Stored degree-factor bits are WR11: raw parameter only. Period-bound
arithmetic is supported for period <=2^24 with exact product/floor; larger values
and unproved inherited numeric modes remain Unsupported. Outer-vector insertion,
allocation failure and general overlap/self-insertion are not added contracts.
No C ABI/plugin/map/default/game/native verification changes.
