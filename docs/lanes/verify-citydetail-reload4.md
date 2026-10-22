| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-citydetail-reload4: CD7 persistence validation | `verify-citydetail-reload4` / engine lane, detached HEAD | ready: both save/reload directions PASS; 3 CLEAN2033/2034/2035 ref0; exact8-byte records; constructed shadow8/8; reused gate cargo4940/0 clippy0 ctest68/68; no engine/ABI changes | Park final logical3/3; CD6/CD8 remain open; no fourth round or algorithm/full-owner exactness claim. |

## Report

Completed the narrowed CD7 persistence task: **native-save → replacement-reload** and **replacement-save → native-reload** both passed in isolated supported fixtures. This report replaces the earlier persistence stop; CD8's natural reduction capture boundary did not block these independent checks.

Only this lane report changed in Git. No plugin/core/capi behavior, C ABI exports/version, maps, defaults, preferences or private fields changed. No game files were committed. Detached HEAD was retained; no merge, push, restart, subagents or real Documents/user-config writes occurred.

## Immutable DLL and reused gate

All three requests used the same frozen `build-runs/dll-cd7-final/FreeCity4.dll`, copied with its PDB from `verify-simgrids-shadow4/build-runs/dll-sgv4-r4`. Full DLL SHA256: `ED12FAE2F0A13F6FDE1A03E4171CD29C7D5CB5EDF264A004C5C0294D17A1E1C1`. Every verdict identifies that path and harness hash `ed12fae2`; the binary was rehashed before every request and after completion.

The citydetail core/capi/plugin/shadow diff against that DLL's gated source `7e9f580e` is empty. Per the explicit instruction to reuse a gated immutable DLL, no blind baseline/final rebuild was run. Re-read its existing qbuild/fc4verify logs: **cargo4940 passed/0 failed** across21 result lines, **clippy clean**, build PASS, **ctest68/68**. Copies of the four logs are retained in this worktree's `build-runs/gate-reused/`. The DLL includes that lane's opt-in simgrids verifier; `simgrids_shadow=0` in all requests here. No Rust files were touched, so fmt is n/a.

## Profiles and isolated save chain

The middle run uses the supplied current40 list verbatim. Both native controls use the same list minus only `citydetail` (39 keys, including `simgrids` and `ports`). `citydetail_shadow=1` on those native sessions; it is0 on the replacement session. Requests are preserved under `build-runs/game-queue/done/`; all use turbo1 and scripted full SaveCity followed by digest. One request was outstanding at a time, labels were unique, and the local helper waited without qwait. There were no failed-start/harness-error retries.

All fixtures are under main `build-runs/fixtures/`. The supported harness `fixture_city` transport stages a writable alias in the isolated slot and records the input SHA256. The helper first observes that exact staged input, then captures the changed complete DBPF after it has stabilized while its game is still active, checks source/copy hashes, and marks the new immutable fixture read-only. It never infers a save from a subsequently reused slot. Each next harness receipt equals the previous captured output hash; all source fixtures were rehashed unchanged.

- Initial input: `sgv4-r1-native39-20261022c.sc4`, 1,007,107 bytes, SHA256 `F03DA3420CD8FABE50BD887890310D2D5577E5C5C109F76E3906EF84E255FB7A`.
- **cd7-r2-native-save-20261022a**: original citydetail, slot2, full save2032-12-01, ok1. CLEAN2033-01-01, ref_warnings0, real_seconds50.297. Preserved output `cd7-r2-native-save-20261022a.sc4`, 1,009,722 bytes, SHA256 `AB2BF8E6D134FCA26167C9250FAF66A2FFF162A42E78758D49D66BEF309D00D1`.
- **cd7-r2-repl-reload-20261022a**: reloads exactly that native output with replacement citydetail, slot1. Full save2033-12-01, ok1 (12,359ms). CLEAN2034-01-01, ref_warnings0, real_seconds50.953. Preserved output `cd7-r2-repl-reload-20261022a.sc4`, 1,003,217 bytes, SHA256 `686902287B0CA90652F69F5DD2EAB6BB0611E2B374498A6F6F79D90662419523`.
- **cd7-r2-native-reload-20261022a**: reloads exactly that replacement output with original citydetail, slot1. Full save2034-12-01, ok1 (11,563ms). CLEAN2035-01-01, ref_warnings0, real_seconds36.094. Additional preserved native output `cd7-r2-native-reload-20261022a.sc4`, 1,007,611 bytes, SHA256 `55FE02243F2D476BEFF07EE320A2778F3EC0730DD63E582A5ABFF41BBECC2856`.

## Exact records, original controls and lifecycle

The initial fixture and all three preserved outputs contain exactly one citydetail class entry `(EAD4BA36,299B2D1B,0)` and one object payload of **exactly8 bytes**: `00 00 00 00 01 00 00 00` (version0, level1). The read-only audit validates DBPF index bounds, class entry framing and DIR compression status before extracting the payload. Object IDs differ between saves (60096548 →343064664 →82840068 →345845848); identity is not inferred from matching IDs. The exact payload is compared, not the DBPF wrapper/CRC or a hash alone.

The replacement log records one object `04F00A00`: `Read ->1 (level1)`; `Init: setting=1 level1->1 pass=none removed=0 created=0 draws=0 network-lots=0`; `Write ->1 (level1)`; `Shutdown ->1`, followed by destruction of that same object. Shutdown records inits1, QueryInterface own1/serializable2/refused0, GetCityDetail0. Thus the actual reload exercised Read before Init and a no-pass persistence lifecycle through final Release.

Both native logs explicitly show the citydetail replacement factory off and contain no replacement citydetail object. Each contains one fresh-original shadow line from public GetClassObject+QI, comparing fresh Init, Read same-level, Read none-level and version1 refusal against Rust. Across both sessions: **8/8 constructed cases exact**, comprising6 record/level comparisons and2 matching Read refusals; mismatches0, missing cases0, incomplete/faulted cases0. Acquired references are released by the existing shadow; all run verdicts report ref_warnings0. The constructed originals are separate from the city's original manager; no unlogged live-manager GetCityDetail call is claimed.

All three complete logs show successful full SaveCity, digest, end-date quit and **PostSystemServiceShutdown**. All verdicts have foreign_exceptions0, watchdog_stalls0, rtc0, overruns0 and ref_warnings0. No turbo-only differences were observed, so no normal-speed rerun was required or claimed.

## Evidence, questions and limits

Local audit helpers: `build-runs/cd7_run.py`, `cd7_record.py`, `cd7_audit.py`. Final parsed evidence: `build-runs/cd7-final-audit.json` (input/output hashes, every log/verdict/request hash, save lines, shadow lines and capture times); each `*-capture.json` records the in-slot/copy hash match. Complete logs/verdicts remain under `build-runs/game-results/`, with citydetail lines also retained in `*-detail-lines.txt`. Final audit PASS covers both persistence directions; no request remains outstanding.

No new question was filed and no spec/question text was rewritten. Existing **CD7** now has measured transport and two-session persistence evidence for this level1 no-pass/current40 fixture chain. This does not prove every detail level, malformed natural saves, increase timing, reduction algorithms or full-owner/domain behavior. **CD6 and CD8 remain open** in `docs/specs/questions/city-detail.md`; native/Rust algorithm comparisons=0. Rust-only dry-run reductions in the existing shadow are explicitly excluded from exactness counts. No preference/field patch was used to force a pass.

Milestones: `4ab60995` scope/handoff; `af45ac49` immutable gate/fixture; `5dacc327` native save control; `68d4551a` native→replacement result; final report records the replacement→native audit. The earlier CD8 boundary audit/question remains in parent `63b0c2bd`. This is final logical round3/3: **park after report/queue; no fourth round**.
