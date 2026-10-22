| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-fontsys4: cGZFontSys verification round 2 | `verify-fontsys4` / detached reserved worktree (engine lane) | ready: cargo4962/clippy0/ctest68 PASS; 8 corrected-artifact runs clean2032/ref0,16 full saves,4 reloads; constructed46240exact/0diff/0refusal,368 route exact; fallback/resync0,natural0; ABI exports changed; no new questions; no whole-owner credit | 1. Coordinator review and merge the default-off probe with these measured-scope limits. |

## Result

Ready for review within the measured constructed FontSys scope. Round 2 verified the corrected actual raw-header provider from 8ff837d9 against independently owned original instances using the production Rust `GzFontSys`, including successful acquisition, cache, range, family, code-page and lifetime cases. No engine source changes were needed this round: all measured comparisons matched. The round added committed evidence/report milestones and preserved immutable artifacts, requests, logs and saved fixtures under build-runs.

This is round 2 of at most 3 lane rounds. The inherited count of three native requests was not a lane-round cap. All three historical attempts remain recorded below; this round added eight justified distinct profile/reload cases, for eleven cumulative game requests. No request is outstanding. No default, map, ABI-table or owner-credit change is requested.

## Stable gate and artifact

- Baseline qbuild all PASS4962/clippy0/ctest68 (90.9s; docs changed during gate). Repeated stable gate at0649002c PASS4962/clippy0/ctest68 (80.6s), no tree-change warning; engine unchanged from8ff837d9.
- Frozen build-runs/dll/fontsys4-r2-corrected: DLL9c7a5fd37144fac68bd2e5557b01b11b2a88746fa814069d5e2cd558d81881b2; PDBf809f731dfa8445218e70eea07165238141ea1f763b15e9e0d6a0ff127f5d4c6; freeze.json includes gate head/hashes, copied gate logs.
Scoped `qbuild fmt` of the verification Rust module passed (absolute file path; the first relative-path invocation could not find the file). The stable gate has 4,962 Rust tests, zero failures/ignored tests, zero clippy diagnostics, and CTest68/68; no engine diff exists from the stable gate. The DLL/PDB hashes remained unchanged across all eight requests.

## Round 2 game results

Each ON run compared load, two presaves, and shutdown: 2,890 exact operations per phase, 1,893 positive returns and 997 zero returns. The native route has 23 additional checks per phase. OFF runs are clean integration controls and receive no 0/0 equivalence credit. All runs use the same frozen artifacts, sim_turbo=1, end_date2032-01-01, full saves and RequestQuit through PostSystemServiceShutdown. The four reloads use the supported fixture_city path and the harness confirms each parent fixture SHA256.

- d1 fontsys4-r2-original-on-20261022d1 clean/ref0 date2032-01-10: four phases each2890exact/0diff/0refusal/0fallback/0resync/1893positive/997zero/natural0; native route23/23 each; full saves2026-03-02(11765ms),2031-06-02(11563ms) ok1. Fixture copied to MAIN/build-runs/fixtures/<label>.sc4; metrics WT/build-runs/<label>-summary.json.
- e1 fontsys4-r2-original-off-20261022e1 clean/ref0 date2032-01-04; probe OFF (no FontSys lines, no 0/0 equivalence credit), two full saves ok1; capture/metrics at matching fixture/summary paths.
- f1 fontsys4-r2-original-on-reload-20261022f1 clean/ref0 date2032-01-05: staged d1 fixture SHA5ca51d83fbd2cb739a0a445915fc13bb09dd0b75b3d88cfbb92bb121eec6e994; loads saved2031-06-02, full saves both2031-06-03 ok1. Four phases each2890exact/0diff/0refusal, route23/23; fallback/resync0/natural0.
- g1 fontsys4-r2-house40-on-20261022g1 clean/ref0 date2032-01-01: four phases each2890exact/0diff/0refusal, route23/23; fallback/resync0/natural0; full saves2026-03-01(12234ms)/2031-06-01(11703ms) ok1, captured fixture. Ports enabled, terrain/dispatch/effects OFF confirmed.
- h1 fontsys4-r2-house40-off-20261022h1 clean/ref0 date2032-01-01; probeOFF/no-equivalence-credit, full saves2026-03-01(11125ms)/2031-06-01(11156ms) ok1, captured fixture. Profile40 includes simgrids/ports, excludes terrain/dispatch/effects.
- i1 fontsys4-r2-house40-on-reload-20261022i1 clean/ref0 date2032-01-01: staged g1 fixture SHAef224fb8a87d4d5d13b8831004cad85e46df1f3c55c6fcf6d2276aeb3221fa55, full saves both2031-06-02(11750/11609ms) ok1; four phases each2890exact/0diff/0refusal; route23/23, fallback/resync0/natural0.
- New corrected-provider ON totals so far final: 16 sequences,46240exact/0diff/0refusal/0fallback/0resync/natural0;368 native-route checks separate. Historical invalid counts excluded.
- j1 fontsys4-r2-house40-off-reload-20261022j1 clean/ref0 date2032-01-01; staged h1 fixture SHA2d44a05076331519cbda3655ecb4cb0c9e815a7ed52c46b641e5ad5388b43a49; full saves both2031-06-02(10906/10875ms) ok1; probeOFF/no-equivalence-credit; complete shutdown.
- k1 fontsys4-r2-original-off-reload-20261022k1 clean/ref0 date2032-01-07; staged e1 fixture SHA8dd895ff65e9082b4557f47a67c7ed4eb342acf3b4343459e0b80fa524a9a467; full saves both2031-06-14(10984/10953ms) ok1; probeOFF/no-equivalence-credit; complete shutdown.
- Evidence audit PASS:8runs2032/ref0,16fullsaves,4stagedreloads,16ON sequences46240exact/0diff/0refusal/0fallback/0resync/natural0;30288positive/15952zero;368native-route exact. JSON build-runs/fontsys4-r2-evidence-audit.json; source diff from stable gate empty.
The evidence audit checked every completed verdict, request profile, probe setting, DLL hash, full-save result, shutdown marker and staged parent-fixture hash. Totals are 46,240 exact / 0 differences / 0 refusals / 0 fallbacks / 0 resyncs, with 30,288 positive and 15,952 zero returns; 368 native-route exact checks remain separate. These are sixteen repetitions of the defined constructed sequence, not 46,240 distinct vectors or natural calls. Historical invalid counts are excluded.

The exact HOUSE40 profile is: landvalue, weather, ordinance, demand, power, plumbing, flammability, pollution, protection, crime, aura, flora, landfill, residential, neighbors, zones, development, tract, history, clock24, commercial, industrial, civic, parks, lotdev, zonedev, traffic, automata, netmgr, netlot, nettool, demolition, occmgr, propmgr, lotcfg, citysit, propdev, simgrids, citydetail, ports. All four HOUSE40 requests use this explicit set; terrain/dispatch/effects remain OFF.

## Measured scope and limits

- Construction, both QI views, unknown-IID out preservation, AddRef/Release, repeated Init/Shutdown/reinit, renderer null/same-pointer replacement and renderer reference counts.
- Directory patterns, missing/repeated directory and path insertion, reverse insertion/path ordering, family/style availability, bounded catalogue records and borrowed-cache enumeration, duplicate/case-sensitive substitutions and sentinel outputs.
- Successful cached and uncached path acquisition, size/style cache hits and pointer identity, family-list selection (empty/missing/quoted/spaced/empty-field/alias cases), kind selection and acquired-font colour.
- System and named monochrome ranges, empty pre-Init family lookup, insert-unique/erase behavior, code-page/AA inheritance versus cache reuse, caller/cache reference counts, FontRelease eviction, family removal, caller survival through Shutdown, and acquisition creating a previously absent catalogue entry.
- Production Rust computes selection, query and cache behavior. A single bootstrap catalogue snapshot supplies the dependency provider's file/face metadata per pair, and actual immutable file headers/table directories supply content probes. No computed acquisition/cache/getter answers are fed back into Rust, no periodic resync occurs, and no private FontSys algorithm is used. Shared metadata does not independently validate a font metadata parser.
- Natural calls remain zero. Each phase constructs a new pair and keeps its Rust model alive for the full sequence; saved-city reload proves integration, not persisted FontSys catalogue state. Font metrics, rasterization/drawing, default platform code-page behavior before explicit setters, and cGZFont FN18-FN22 are outside this measurement. No whole-owner or whole-renderer verification/replacement credit is claimed. No turbo difference occurred, so no normal-speed difference investigation was needed.

Evidence paths: WT/build-runs/game-results/<label>.verdict and .log; WT/build-runs/game-queue/done/<label>.req; WT/build-runs/<label>-summary.json; WT/build-runs/fontsys4-r2-evidence-audit.json; frozen DLL/PDB/gate logs at WT/build-runs/dll/fontsys4-r2-corrected/. Reload fixtures remain in MAIN/build-runs/fixtures/. Helpers fontsys4-submit.py, fontsys4-wait-verdict.py, fontsys4-collect.py and fontsys4-audit-evidence.py remain in WT/build-runs. Labels are unique; no qwait was used, no completed verdict discarded, and only one request was outstanding at a time.

Questions: none new; no unresolved native difference was found in the measured domain. Existing FN18-FN22 remain excluded. The overall lane adds three verification-only C exports (ABI=true); no ABI-version bump or public ABI-table edit. This round changed only the lane report among tracked files. Guard manifest remains C:/Users/tlori/FreeCity4/build-runs/run4-verify-fontsys4-r2-manifest.json; no counters or refs were reset.

## Prior round evidence (historical)

Round 1 stopped WIP after three native requests and corrected the provider in 8ff837d9. Its acquisition-provider setup evidence is invalid. Round 2 above supplies fresh corrected-artifact results; it does not convert those historical counts into exactness credit.

## Implementation

- Scope: drawing-fonts sections 1/12/12.1, actual production `GzFontSys`, `FontFixedInfo`, `FontInfoRecord`. No core algorithm changes, private verification clone, delegated selection, native-query answer injection, or periodic model resync. cGZFont FN18-FN22 excluded.
- Opt-in `fontsys_verify = 1`, default off. Observer load/presave/shutdown hooks run constructed owned instances; natural instrumentation count is explicitly zero. Each comparison retains a production Rust object through its command sequence, then destroys that pair.
- Original route: GZCOM creates CLSID 0x20673960 / service IID 0x287FB697, primary QI 0x00673960, primary vtable checked against published 0x00AC5488 relocated from module base. QI views differ by four bytes. Acquired views are balanced with audited created-reference releases. No global FontSys getter or global-service Shutdown.
- Explicit published slots 9/10/11/13/14 avoid relying on MSVC overloaded virtual declaration ordering. Font getters use published slots 7/10 where necessary. No guessed private getters or shared ABI/checker changes.
- A single bootstrap catalogue snapshot per constructed pair provides dependency-provider metadata for 344 read-only Windows fonts. Production Rust populates its own catalogue and computes queries. Corrected adapter additionally captures raw font-header/table-directory bytes once at construction; subsequent content probes use immutable bytes. Provider metrics/raster behavior is outside this FontSys measurement domain.
- Added three verification-only C exports (`fc4_fontsys_verify_new`, `fc4_fontsys_verify_call`, `fc4_fontsys_verify_free`). ABI flag true; ABI version, public headers/tables and defaults unchanged. No binaries committed.
- Main work commits: d916e22d owned-route probe; 7c84f857 production bridge (before call 30); 5067dab6 explicit overloaded slots; bc7d8149 directory/renderer coverage; 8ff837d9 raw-header provider correction and regression.

## Historical game requests

All three historical requests used unique labels and all completed verdicts were retained. Their request count is three, not a separate request cap. No historical request is outstanding.

1. `fontsys4-route-20261022a1`: clean/ref_warnings=0, 2035-01-03. Load and shutdown each route exact=23, diff=0, refusal=0, fallback=0, resync=0, natural=0. Initial request placed end_date under ini, so the seconds limit ended it after 2032; not relabeled as a fixed-date control.
2. `fontsys4-focused-20261022b1`: exited -1 before production results during bootstrap AddFont through an overloaded SDK call. No reported foreign exception, Rust panic or reference warning. Corrected call shape to explicit published slots; do not count this as passing conformance.
3. `fontsys4-stable-20261022c1`: clean/ref_warnings=0, no foreign exceptions/stalls/RTC/panics/structured exceptions/other warnings, date 2032-01-05. Full save requested for 2026-03-01 ran on 2026-03-15, `ok=1`, 11641 ms. Each load/presave/shutdown phase: production exact=2823, diff=24, refusal=43, fallback=0, resync=0, natural=0, metadata_once=344, positive=1869, zero=978. Each phase also has 23/23 native route checks, counted separately.

Historical raw production counts: 8670 operations = 8469 reported exact + 72 differences + 129 refusals; fallback=0, resync=0, natural=0. INVALID acquisition-provider setup evidence: no exactness credit and no inclusion in round 2 totals. The early exited overloaded-SDK case remains a failed attempt.

The immutable tested DLL and PDB remain in `build-runs/dll/fontsys4-stable/`; DLL SHA256 `A7D4536F5F3A775EC9818BAB3A697000532E95951058D144D2815F8853B3FC48`. This binary predates the provider correction. Logs/verdicts are in `build-runs/game-results/` under the labels above.

## Failure diagnosis and limits

The old provider returned empty `read_head` data. Production `GzFont::init` correctly refused that content probe even though native opened the real font (first example `C:\WINDOWS\Fonts\AGENCYB.TTF`); downstream handle-dependent comparisons refused. Treat those differences as an invalid provider setup, not a discovered native algorithm or spec discrepancy. Correction 8ff837d9 snapshots real read-only headers and has a Windows integration test proving actual production Arial acquisition, family-cache reuse, codepage propagation and reference/cache release. It has no real-game equivalence verdict.

No fourth original attempt or normal-speed comparison was made. Original/own/current40 immutable control set and saved-fixture reload are incomplete; the successful 2032 save/shutdown run alone does not establish them. No current40 preset or reload script action was found in the allowed operational docs/engine. No promotion, default change, whole-owner coverage credit, or claim about untested paths.

## Gates and questions

- Initial baseline: Rust tests/clippy passed, ctest 68/68. Its native build already included the default-off route probe.
- Stable pre-correction full gate: qbuild all PASS, 4961 Rust tests, clippy zero diagnostics, ctest 68/68. Summary `build-runs/fontsys4-finalgate-summary.txt`.
- Corrected-provider quick gate: PASS, capi 531 tests / 0 failed / 0 ignored and native build PASS; summary `build-runs/fontsys4-provider-quick-summary.txt`.
- Corrected-provider final full gate: qbuild all PASS, 4962 Rust tests / 0 failed / 0 ignored, clippy zero diagnostics, ctest 68/68 (94.2 seconds), native build PASS. Summary `build-runs/fontsys4-corrected-finalgate-summary.txt`. Wrapper notes tree changed during the run: only this lane report changed; engine sources remained committed at 8ff837d9 (confirmed by final diff). This is not a fresh native conformance verdict.
- Corrected built DLL SHA256 `9C7A5FD37144FAC68BD2E5557B01B11B2A88746FA814069D5E2CD558D81881B2` was not game-tested at the round 1 stop; round 2 froze and tested this hash after a stable full gate. Preserve the distinct frozen pre-correction DLL and its evidence.
- New Rust module passes qbuild fmt. Root lib.rs traversal reports 645 existing formatting diff lines in untouched children; scoped root check also identifies pre-existing test-module ordering. No unrelated reformat applied.
- Questions: none. The identified failures are adapter defects, not missing native contracts; no speculative question filed. FN18-FN22 remain excluded.

## Next steps

Coordinator review and merge the default-off probe and bounded verification report. Required corrected-artifact native comparisons, original/HOUSE40 ON/OFF full saves/shutdown, and supported reload controls are complete. Preserve historical failed/invalid evidence and the measured-scope limits; no default, map or owner promotion is requested. No further request or lane round is needed for this brief.
