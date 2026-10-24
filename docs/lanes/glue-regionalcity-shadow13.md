| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-regionalcity-shadow13 (regional city shadow) | Codex / `glue-regionalcity-shadow13` (engine lane) | READY; cargo 7409/0/0, clippy 0 diagnostics, CTest 81/81; run 634 clean, ref_warnings=0, regionalcity 73/73 exact through 2032; RY8 filed; no public C ABI change | (1) Land and append regionalcity_shadow to the current single regress shadows string; credit coverage. (2) Regenerate question index and reconcile RY8 wording. |

## Report

Added the default-off `regionalcity_shadow` observer and private static-link comparison bridge. At city load and monthly it enumerates the loaded region plus the current regional-city object, captures each native record through its public serialization interface, decodes the same inputs into the core model, and compares native getters field by field. Coverage includes identities and names, location/size/world coordinates, population/jobs/taxes/growth statistics, budget/income, commodities/balances, connections and local-deal counts. Byte-identical record roundtrips additionally cover save-derived fields, deal/connection records, POIs, and strings without getters. It does not refresh the city cache or replace a game class.

Implementation milestones: observer/bridge `93cfd16e3`; SDK string implementation fix `f95ee4ee6`; return-channel review `7842bb59f`; balance precision and load-date correction `386435447`. The new core balance applies foundations §18.3 rounding after each sum and the final subtraction while leaving loaded doubles unrounded. The precision regression checks operand loading, intermediate rounding, and the observed fractional discrepancy. Load timestamps now use the simulator date rather than Julian zero. Public C ABI exports and version are unchanged.

The final stable-source `qbuild all` passed: **7409 passed, 0 failed, 0 ignored** over 51 workspace summaries; clippy `-D warnings` **0 diagnostics**; native build **165/165 edges**, 188.5 s; CTest **81/81**, 258.6 s. Native queue `qbuild-20261025-000629-build-ctest` terminated PASS, total 449.3 s; outer operation 3983 was collected to its terminal result. Owned Rust formatting passed (two files, zero diff lines); focused precision test passed 1/1. Logs: `build-runs/regionalcity-r3-all-transcript.log`, `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`, and `regionalcity-r3-focused.log`. Earlier failed build evidence remains under `build-runs/regionalcity-native-*`; invocation failures are recorded in `build-runs/regionalcity-r3-input-failures.txt`.

Final real-game evidence, [run 634](../runs/regionalcity13-r3-exact-2032-20261025a-634.md):

```text
regionalcity13-r3-exact-2032-20261025a 73/73
```

Konradshohe, turbo, DLL SHA256 prefix `5e84247c`: clean, `ref_warnings=0`, foreign exceptions/RTC/watchdog stalls/overruns all zero. Final date 2032-01-10, 82.110 real seconds. Every observation decoded all 85 native records and compared 15555 fields; **1135515 total fields**, load plus 72 months, **zero mismatches on every line**, zero resynced lines. Load now logs 2026-01-27, last monthly comparison 2032-01-01. Main-thread x87 control word at PostCityInit is `0x007F` (24-bit, nearest). Logs and verdict remain in `build-runs/game-results/regionalcity13-r3-exact-2032-20261025a.{log,verdict}`. No game or gate operation remains outstanding.

Preserved pre-correction evidence: [run 632](../runs/regionalcity13-r2-2032-20261024a-632.md) `regionalcity13-r2-2032-20261024a 61/73`, turbo; [run 633](../runs/regionalcity13-r2-normal-20261024b-633.md) `regionalcity13-r2-normal-20261024b 63/73`, normal. Both clean/ref_warnings=0 through 2032; their only differences were commodity balance rounding. They are distinct from the final corrected-source run.

Filed **RY8** in `docs/specs/questions/regional-city.md`: reconcile regional-city §5's double-precision wording with foundations §18.2–18.3. The literal foundations interpretation is documented in code and now measures exact. This wording question blocks no measured portion of the shadow.

Keep the key default-off. Clean-room restrictions and the detached-head transport prohibit this lane from editing `tools/` or merging main; the coordinator must append `regionalcity_shadow` to the current single regress shadows string after landing and credit the exact run in coverage. Coverage maps and regress tooling were untouched. Regenerate the question index from the appended question. All implementation and evidence records are committed; the supervisor publishes this detached tip to the reserved lane branch.
