| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-propcfg4 (prop and lot independent verification) | `verify-propcfg4` / (engine lane, detached HEAD) | ready: code ff4388f3/99d9b5aa, report1113f298 finalized; cargo4938/4938, clippy0, ctest68/68; six clean2032/ref0 full-save/shutdown controls including own/current40 reloads; native exact/total catalogue3318/4862, prop queries11840/11840, create2336/2336, lot queries9354/9354 turbo and9361/9361 normal, fixtures5/5, arrays1/1909, C++ getters87814/87814; PL23 padding1544/run and arrays1908/run retained; raw shadow NOT exact; ABI unchanged | 1. Coordinator reviews and integrates the queued opt-in delivery. 2. Resolve PL23 through published specs; no whole-configuration coverage promotion. |

## Delivery

Final round3/3 closes the already committed delivery. Reviewed HEAD entering this round was `68fe7c7707cfe197d8ba7630d8e5965fb8945e46` (handoff after report `1113f298`); implementation commits remain `ff4388f3` and `99d9b5aa`. The ready report is committed on top of that detached HEAD. The queue's `tip` records the actual full delivery HEAD obtained after that commit, rather than a branch ref or a predicted hash. Queue: `build-runs/merge-queue/verify-propcfg4.json`, reserved branch `verify-propcfg4`, kind `engine`, state `ready`, ABI exports unchanged. All six unique labels below are included.

This round only finalizes documentation and the queue; no new source, build or game run was needed. No request is outstanding. No RE, maps, defaults, tools or ABI tables changed; no external communication, restart, merge or push was performed. Evidence, immutable DLL and save receipts remain in ignored `build-runs/`; no game files are committed.

## Implementation and scope

Code commits: ff4388f3 (independent loading), 99d9b5aa (live inputs, fixtures and evidence). `FC4PropLotShadow.cpp/.h` keep the existing opt-in `propmgr_shadow` and `lotcfg_shadow` switches. The Rust catalogue now loads resource configurations independently through `cFC4LotConfiguration`; original configurations no longer supply Load outcomes, families, size or objects to its host. Queries compare complete ordered results and return values, including missing keys and repeated set calls. Duplicate/missing/null object-expansion fixtures are separately labelled. Prop creation checks real uninserted native occupants, released after comparison.

Counters report every pair, difference and refusal, with fallback/resync zero. Replaced owners explicitly refuse native comparison; they never earn zero-call equality credit. Record roundtrips disclose the native Write bootstrap and receive no algorithmic or independent-evolution credit. Reference comparison now walks the whole capture instead of stopping at 63 identities. Probes use a private LCG and draw no game random values: weighted random selection is outside these owners' published contracts.

All probes are diagnostic calls (`natural_calls=0`), including probes with live city inputs. No simulation-method interception or whole-owner native shadow is claimed. Resource bytes are inputs, not algorithmic verification results. Configuration getter checks are **C++**, and full configuration loading uses C++ with the Rust object decoder; neither is a Rust implementation of the complete configuration owner. Their reported native pointer anchors the original manager/map context, not a separately identified configuration instance. No configuration-owner promotion is requested.

## Gate

Baseline and final `qbuild all` passed cargo4938/4938, clippy0, CTest68/68. Final stable gate at code99d9b5aa: CTest81.91s, no tree-change warning; no source changes afterwards. Focused `qbuild quick` passed after substantive additions. No Rust files touched, so rustfmt is not applicable. Final `git diff --check` clean. Logs and baseline/final snapshots: `build-runs/evidence/`; Bash q.py was unavailable (E_ACCESSDENIED), so capped PowerShell reads were used. No ABI/table/default/tool changes.

All games used immutable `build-runs/dll/propcfg4-r2-v1/FreeCity4.dll`, SHA256 `04b93bd72ee847fad54084520d5492c48518176e6746d8b31a1dc2be357339db`.

## Native same-process results

| Operation | Turbo exact/total | Normal exact/total | Differences |
|---|---|---|---|
| Prop catalogue, ordered families and missing outcomes | 3318/4862 | 3318/4862 | 1544 raw profile-padding differences each |
| Prop live-city ordered queries | 11840/11840 | 11840/11840 | 0 |
| Prop CreateProp outcomes/class/type/box | 2336/2336 | 2336/2336 | 0 |
| Lot manager ordered resource queries | 9354/9354 | 9361/9361 | 0 |
| Constructed duplicate/missing/null expansion fixtures | 5/5 | 5/5 | 0; building/prop/null, no flora fixture credit |
| Configuration complete ordered arrays (C++/Rust decoder) | 1/1909 | 1/1909 | 1908 each, no foundation removal or normalization |
| Configuration supported getters (C++) | 87814/87814 | 87814/87814 | 0; no complete-owner credit |

Every native comparison scope in this table has `refused=0`, `fallback=0`, `resync=0`, `natural_calls=0`, `copied_record_credit=0` and `native_random_draws=0`. Per run, report counts are catalogue1, live prop queries74, prop creation73, lot queries74, expansion fixtures1, complete arrays1 and supported getters1. Counts are diagnostic pair totals, not intercepted simulation calls or independent evolution. Catalogue inputs contain1544/1544 types and112/112 families (native/ours), with1136 family members; keys, validity, specified profile fields and ordered families have zero differences, while all1544 raw-padding differences remain counted. Complete arrays cover1909 configurations and25210 objects at detail1, with1908 foundations and0 unbound objects.

At each speed the legacy line summaries are prop221/222 exact (1 mismatched,0 incomplete) and lot296/297 exact (1 mismatched,0 incomplete). Those line-level fractions are distinct from the pair-level denominators above and do not erase raw differences. For each manager at each speed, all74/74 native-Write roundtrips report read=1, consumed=1, init=1 and differences=0: read failures0/74, unconsumed records0/74 and initialization failures0/74. These are bootstrapped record checks with algorithmic_credit=0 and independent_evolution=0, not independently generated saved-state equivalence.

Each native run also compared74/74 prop and74/74 lot-manager record roundtrips with zero read/consume failures, separately from algorithmic evidence. Turbo/normal live inputs: lot cells1618/1619, returned props2232/2224, lot configurations308/312, object expansions1770/1765. Zone/pricezone calls succeeded but accepted0/16 cells; no zoning-mutation credit. Native manager pointers: turbo prop1443E514/lot1443E4D4, normal prop1447E514/lot1447E4D4. Source is the city's unreplaced interfaces; runtime IsOurObject checks also refuse replacements.

## Game controls and persistence

| Unique label | Profile / loaded date | Verdict date | Result |
|---|---|---|---|
| vpc4-r2-native-20261022a | Original, turbo, fresh2026 | 2032-01-02 | clean/ref0 |
| vpc4-r2-normal-20261022a | Original, normal speed, fresh2026 | 2032-01-01 | clean/ref0 |
| vpc4-r2-own-20261022a | propmgr, lotcfg; fresh2026 | 2032-01-12 | clean/ref0 |
| vpc4-r2-own-reload-20261022a | propmgr, lotcfg; saved2031-10-18 | 2032-01-18 | clean/ref0 |
| vpc4-r2-current40-20261022a | Literal current40; fresh2026 | 2032-01-01 | clean/ref0 |
| vpc4-r2-current40-reload-20261022a | Literal current40; saved2031-10-07 | 2032-01-01 | clean/ref0 |

All six executed full SaveCity successfully and quit through the full shutdown verdict. All four replacement controls refused two native shadows each: **no** replacement self-comparison or zero-pair exact credit. Current40 controls report switches_off=- and disabled=-. Runtime current40 factories created cSC4LotConfigManager0C573988 and cSC4PropManager03819D58; lotcfg Init loaded1909 configurations and282 families at detail1. Reload again loaded1909/282, and the observer recorded the expected saved date before advancing. This tests persistence and shutdown, not cross-process simulation equality.

Replacement refusal scope is one refusal per owner per run: each reports pairs=0, exact=0, refused=1, fallback=0 and resync=0. Thus the four own/current40 fresh/reload controls total8 refusals and0 native comparison pairs; their clean verdicts earn persistence/shutdown credit only. All six final verdicts separately report foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0 and ref_warnings=0, with switches_off=- and disabled=-. Full-save success is6/6; saved-city reload controls are2/2 (own and current40), not six reloads. No failure-injection, whole-owner equality or cross-process simulation-equality claim is made.

Literal profile: landvalue, weather, ordinance, demand, power, plumbing, flammability, pollution, protection, crime, aura, flora, landfill, residential, neighbors, zones, development, tract, history, clock24, commercial, industrial, civic, parks, lotdev, zonedev, traffic, automata, netmgr, netlot, nettool, demolition, occmgr, propmgr, lotcfg, citysit, propdev, simgrids, citydetail, ports.

Saved DBPF fixtures were captured from the isolated `build-runs/sc4slots` profile before reuse, never from real Documents. Own fixture:1004051B, SHA256 `fbb6358d381ee054aba8e75e48076f949bef4f0881c0083f4b3a033c62bbf32d`; current40:1002130B, SHA256 `cd04d4e952df14cf5b4e021d551d9a5ba7b0eb4d590ef388a0771f4435ae668f`. Reload verdicts record those exact hashes and preserved sources. Files are under main `build-runs/fixtures/<fresh-label>-saved.sc4`; capture receipts, request scripts and per-run summaries are in this lane's ignored `build-runs/`. No game file is committed. Full logs/verdicts: `build-runs/game-results/<label>.*`; no request outstanding.

## Questions and limits

PL23 appended to `docs/specs/questions/prop-lotconfig.md`: retain all1544 raw profile-padding differences and all1908 array differences per native run. The published specs leave profile padding and foundation IDs unspecified, with zero chosen in FreeCity4. First array differences are the last records and foundation counts1908, but no field-by-field diagnostic rules out other differences. **Raw shadow is not exact**, at either speed. No defaults changed to match native memory. Existing PL19-22 stay parked. Question-index regeneration is left to the coordinator because this lane may only append its question file and may not run tools/ scripts.

Unsupported configuration name/roads/fences/building-info/editor-save paths are explicitly logged. Filter caches, footprints, editing, detail-change/two-city transitions and capped glue21 work remain outside this verification subset. Next: review this opt-in verification delivery; resolve parked contracts through published specs only. No whole-owner coverage promotion, restart, merge or push.
