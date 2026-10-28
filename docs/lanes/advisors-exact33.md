| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| advisors-exact33; engine; citysit native string and hash order | Codex; advisors-exact33 worktree/reserved branch; detached HEAD; code 635cbaafa, 38d9359b3 | WIP; no newly exact owner; last comparable citysit 3/4 exact, all clean; final qcheck lacks references; cargo 8411 passed, clippy 0 diagnostics, CTest 146/146; ABI unchanged; CS19 follow-up, CS20 | 1. Refresh the schedule pin and references through the coordinator. 2. Rerun citysit and identical original-only controls. 3. Fix advisors and standalone mysims. |

635cbaafa constructs the native slot-16 target-name result with allocator B; Lua uses a separate constructed-string helper. Diagnostic qcheck-22 showed a zero output vtable with no active advice/target. CS19 records the evidence; the construction contract remains SPEC-GAP.
38d9359b3 replaces the success-count BTreeMap with the existing STLport identity-hash map. Duplicate reads keep their first value and profile rewards overwrite in place; the C API enumeration stays ascending. A new collision/overwrite regression passes. CS20 asks for the missing bucket/rehash contract; the default hint 100/193 buckets is SPEC-GAP.
Inherited Lua publication, hexadecimal GUID conversion, tracking and saved advisor identities remain. No original citysit implementation is forwarded. No C export or schedule file changed this round.
Final qbuild all PASS on the hash-map source: cargo 8411/0, clippy 0 diagnostics, native build 10/10 edges, CTest 146/146. Logs: build-runs/qbuild-{test,clippy}.log and fc4verify-{build,ctest}.log. Fmt checks passed on all five changed Rust files; git diff --check passed.
The earlier map-change quick run FAILED to compile because CitySituationState derives PartialEq; deriving it for StlHashMap fixed that before the passing full gate. Do not use that failed run as a final-source result.
citysit NOT YET EXACT: final qcheck-25.log exits 2, all four scenarios ERROR/no golden baseline; no game requests ran. The final hash-map DLL has no comparison result.
Last comparable qcheck-23.log (635cbaafa, before the map fix) exits 1: gen28-konradshohe-s2-churn, paradis-jinae and timbuktu-rush-hour exact; maxisland-east-weburg differs only in 0BB14381/299B2D1B/0 (79-byte record, ten success counts). Both former foreign exceptions are gone.
qcheck-23 labels, churn/east/jinae/rush: golden-88f4e29283c14d4a9b2d910fbb4109fe, golden-a953384ff2cb4782a41d50c496473c91, golden-059909302125419ab26d41d23b49c4af, golden-0c0fe8605b9e403e9ef0fdcdf67adbc2.
All four qcheck-23 verdicts clean; foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0. No shadow exactness claimed.
Original-only qcheck-24.log exits 0, 4/4 exact on those same scenarios, using the pre-map DLL with citysit disabled and the earlier reference pin. Final-source controls must be repeated after references return.
advisors NOT EXACT, unchanged/not rerun: qcheck-4 exit 1, 0/4. Rivertner 70 different/51 relabelled, shift 44235/49B9E602; Mitras 63/46, shift 195187/49B9E602; first content neighbour deal 2A134F42 in both.
Advisors Rush Hour: first rand delivery 2010-05-03 #5, reference vt 00AB2214; 101/51 relabelled, shift 4017/2977AA47, first content occmgr. Bevopolis 71/45, shift 417/896E75AF, first content civic.
mysims NOT EXACT/NOT ACTIVATED, unchanged/not rerun: qcheck-6 exit 2 on gen28-tegel-s0-churn and ui29-house. Resolve disables standalone mysims; factories require CurrentAdvisorCore. It needs a host compatible with native advisors.

## Schedule request
qcheck-25 says references are being re-recorded from 05:02 on 2026-10-28 for schedule code bde008a451b1c7fb, main commit c17dfc7c6 (ScheduleQpc export and neighbour ordering). Its automatic note asks for a main merge; this task does not authorize one, so the detached checkout and schedule were left intact.
Coordinator action: supply/authorize the refreshed pin and complete its references, then rerun citysit and the identical original-only scenarios. No owner-driven schedule change is requested.
Next: prove the hash-map fix with qcheck after the pin refresh; only then claim citysit credit. Continue advisors at its first shifted save ID and wire standalone mysims. Queue remains WIP because no complete owner has earned exactness.
