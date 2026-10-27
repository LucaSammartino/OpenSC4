| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| method-lighting31 (engine method evidence) | Codex / method-lighting31 / detached HEAD; supervisor publishes reserved branch | Ready scoped change, code 8c9b32bd2; cargo 8379/0, clippy 0, CTest 144/144; game clean; both lane shadows exact; ABI unchanged; questions none; whole owners remain unverified | 1. Publish detached tip and attach checked method credit. 2. Add ordered entry/state/collaborator observations for the remaining reached methods. |
Gate: qbuild all PASS on stable source; owned Rust fmt PASS. Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log; method_evidence test covers faulted/partial entries and persistent mismatches.
Game: suite-method-lighting31-r1a-gen27-konradshohe-s1-default-shadows, house shadows profile + reach + turbo; clean to 2028-01-28, ref_warnings/foreign_exceptions/watchdog/rtc/overruns all 0; DLL SHA256 prefix d0fcb30e.
Evidence root: C:/Users/tlori/FreeCity4/build-runs/suite/method-lighting31-r1a/ (gen27-konradshohe-s1-default.log, .reach.tsv, .verdict, per_key.tsv); initial suite wait expired, same request completed and was collected with resume.
| Compared method (VA) | Bytes | Calls | Complete | Mismatches | Natural reach entries |
|---|---:|---:|---:|---:|---:|
| Lighting GetShadowParams 0x007D6B70 | 57 | 25 | 0 | 0 | 800 |
| Lighting GetDecalShadowParams 0x007DA490 | 209 | 1625 | 0 | 0 | 0 |
| Lighting CalcShadowProjection 0x007D8C80 | 2964 | 50 | 50 | 0 | 0 |
| Construction GetPosition 0x005CA7D0 | 69 | 62 | 62 | 0 | 683 |
| Construction GetBoundingBox 0x005CA820 | 63 | 62 | 0 | 0 | 672 |
| Construction GetGZCLSID 0x005D04A0 | 6 | 62 | 62 | 0 | 195 |
| Construction QueryInterface 0x005CA7A0 | 39 | 124 | 0 | 0 | 800 |
Method evidence: 175 monthly lines, monotone counters, zero mismatch/resync lines; 3 complete methods / 3039 candidate bytes. Projection is ONLY synthetic count-zero return + all 64 untouched bytes (flags 0/7); no natural projection entry observed. Naturally reached complete methods total 75 bytes.
Shadow exactness: lighting 25/25 captures (15600 fields); construction 25/25 captures (62 live samples, 372 fields). Counts are probe comparisons, not all natural calls; no comparisons were removed or resynced.
Full per-method table, including every remaining address/byte size/entry count/observation limit: [build-runs/method-inventory.tsv](../../build-runs/method-inventory.tsv). Owners reconcile to lighting 48/22577 B and construction 79/26539 B; natural reach 21/48 and 54/79 respectively (capped lower bounds, omitted intervals are not proof of absence).
Largest reached lighting methods left, VA:bytes:entries: 007D9820:1488:168 SetTimeDependentLightingValues; 007D8550:1476:800 GetModelLight; 007DBCF0:1197:179 DoMessage; 007DB840:1170:25 DoZoomAndRotationChange; 007D7F00:1086:760 GetTerrainVertexColors; 007D7BF0:775:800 GetVertexColors; 007D7460:709:800 GetColor; 007D9DF0:682:168 SetTimeOfDay.
Largest reached construction methods left, VA:bytes:entries: 005CAD60:2633:133 RebuildModel; 005CD710:1640:3 AttemptAddOuterCrane; 005CC820:1350:7 CreateCrane; 005CC410:1027:800 Draw; 005CC020:939:486 SetScaffoldHeight; 005D00F0:937:126 constructor; 005D0CE0:934:122 Shutdown; 005D0890:810:126 SetLot.
Lighting missing observations: LT14 ordered tint/time/settings/view/terrain and effective shared-property history, query arguments/maps/terrain dependency results, owner-field writes and collaborator notifications; nonnull decal holder-call transcript. Slots14/16 stay complete=0; nonempty projection unavailable.
Construction missing observations: passive entry inputs, schedule/mesh/renderer/effect/reference mutations and collaborator-call transcripts (LR14/T79); independent record-6 box for GetBoundingBox; QI counter writes/delegation/AddRef. No mutating native methods were invoked as getter probes; all remaining methods have limits in the table.
Coverage baseline: lighting verified 0%; coverage.py with the raw plugin log as --regress reports method-verified 0 (not a canonical regress wrapper). No coverage map edits or owner-credit claims; coordinator must attach/check the explicit log evidence. Prior Write round trips do not verify the construction lifecycle.
Suite-wide result is FAIL, separately from the clean game and exact lane families: actions 462 ok/482 failed, all shadows 6046/6255 exact (209 other nonexact samples); summary.tsv/per_key.tsv preserve those findings. No second game request or pending request remains.
Next: land the scoped counters only; whole-owner verification remains missing. Natural reach prioritizes the methods above; no new spec questions or speculative behavior were introduced.
