| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lighting-shadow9: lighting parameter shadow | Codex, `glue-lighting-shadow9` | ready; default-off partial shadow, gate PASS, house 73/73 exact and clean | Land scoped shadow; resolve LT14/LT15 before broader lighting credit. |

## Changes and scope

Implementation 9e34944a1 adds `lighting_shadow=1`, default off, comparing 14 scalar fields from native lighting slot 14 (`GetShadowParams`) and slot 16 with a null holder (`GetDecalShadowParams`). It captures once per month, including the load month, and reports bitwise comparisons against independent Rust expectations. The native owner is borrowed only for each capture; there are no native setters, lifecycle calls, hardware observers, hooks, or output-seeded expectations.

Diagnostic revision fad1dbe42 reads the six parameter input properties once at city load through the public resource/property interfaces from resource `6534284A/A9189CF0/00000001`, feeding Rust `Parameters::initial().read(inputs)`. Only scalar input copies survive capture; the local exemplar releases its acquired resource. No output resynchronization occurs. No C ABI exports changed and no ABI version was bumped.

LT14 in `docs/specs/questions/lighting.md` requests a complete ordered external-input replay contract. Slots 0–13, 15, 17–29 and slot 16 with a nonnull holder remain uncompared, including dynamic time-of-day, tint/reset, render/view, and terrain behavior. The `cImageARGB8` owner is also outside this parameter shadow. Whole-owner verification/replacement credit is not claimed.

LT15 in the same file records the shipped decal texture discrepancy: the spec table says `69D3BC3B`, whereas independently captured property `085489C8` and the native output both say `69D3A23B`. The initial literal-table failure is preserved. Final measurements compare against the independently read input, so they do not establish the disputed literal table value. LT14/LT15 remain open; the coordinator maintains the question index and coverage credit.

## Validation

Final source fad1dbe42: focused lighting shadow tests 3/3; explicit-owned-file formatting PASS; full `qbuild all` PASS, Rust workspace 6987/6987 (sum of test-result lines), clippy 0 diagnostics, native build 207/207 edges, CTest 79/79. Job `qbuild-20261024-090555-build-ctest` / session 8711 was collected to terminal PASS. Logs remain in `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, and `fc4verify-ctest.log`. Earlier pre-diagnostic source also passed 6986 tests and 79 CTests. No source changed or build was repeated in round 2.

## Real-game evidence

- [Run 469](../runs/lighting9-house-2032-r1-20261024-469.md), `lighting9-house-2032-r1-20261024`: clean, ref_warnings=0, stopped at 2029-10-22 after 601.828 s. Literal-table version 0/46 exact, solely texture field 5 differing on every capture; other 13 fields exact. Led to LT15 and independent input capture.
- [Run 470](../runs/lighting9-house-2032-r2-20261024-470.md), `lighting9-house-2032-r2-20261024`: final source, 73/73 captures and 1022/1022 scalar fields exact through 2032-01-01, 907.469 s. Native verdict clean but ref_warnings=1; not accepted for safety. Warning was in `WriteCapture::ClearReferences:79` after an automataowners construction round-trip failed for unavailable reference IID `2977AA49`, class `A9BC9AB6`.
- [Run 471](../runs/lighting9-house-2032-r3-20261024-471.md), `lighting9-house-2032-r3-20261024`: unchanged DLL, 51/51 exact (714/714 fields), last monthly capture 2030-03-01. Recovered watchdog stall and harness fallback at wall 600 s without a clean shutdown verdict; not accepted.
- [Run 472](../runs/lighting9-house-2032-r4-20261024-472.md), `lighting9-house-2032-r4-20261024`: accepted unchanged final DLL (hash prefix `54fbdfa7`), exact house switches plus lighting_shadow=1, turbo through 2032-01-01, 786.375 real seconds, wall 805 s. **73/73 monthly captures, 1022/1022 fields exact**, captured=1 throughout; shutdown reports 73/73 exact, resyncs=0. Verdict clean; foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0. Input presence=2F, texture=69D3A23B. Verdict and log remain under `build-runs/game-results/lighting9-house-2032-r4-20261024.*`.

Round 2 tried the original wait session 14011, which the transport no longer recognized. It waited on the same r4 verdict path without qwait or resubmission: replacement session 78636 timed out, then session 63971 completed successfully. No game requests remain outstanding. The texture difference was input/spec-related, not turbo-specific; final independent-input comparisons are exact under turbo.

## Landing and next steps

Ready to land the default-off 14-field parameter shadow only. Queue uses the full actual detached HEAD; the supervisor publishes commits to the reserved lane branch. Broader owner comparison needs LT14's independent ordered input contract; LT15 still needs a spec resolution. All prior failed/incomplete evidence remains distinct from the accepted r4 result.
