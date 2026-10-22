| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-fontsys4: cGZFontSys verification round 2 | `verify-fontsys4` / detached reserved worktree (engine lane) | in progress | 1. Stable full gate and freeze corrected provider. 2. Native production comparisons, HOUSE40 OFF/ON save/shutdown and supported reload. |

## Handoff

- Goal: round 2 of at most 3 lane rounds; meaningful independent native comparison using production Rust FontSys and corrected raw provider.
- Starting HEAD 391bfd5d; provider correction 8ff837d9. Detached HEAD, no branch/ref changes authorized.
- Prior native requests retained: route a1 clean, focused b1 early SDK overload exit, stable c1 clean; 8469 exact/72 diff/129 refusals INVALID acquisition-provider evidence, no exact credit.
- User clarified native request count 3 is not a lane-round cap; justified corrected-artifact requests authorized. No separate original-attempt cap applies.
- Baseline qbuild all PASS4962/clippy0/ctest68 (90.9s; docs changed during gate). Repeated stable gate at0649002c PASS4962/clippy0/ctest68 (80.6s), no tree-change warning; engine unchanged from8ff837d9.
- Frozen build-runs/dll/fontsys4-r2-corrected: DLL9c7a5fd37144fac68bd2e5557b01b11b2a88746fa814069d5e2cd558d81881b2; PDBf809f731dfa8445218e70eea07165238141ea1f763b15e9e0d6a0ff127f5d4c6; freeze.json includes gate head/hashes, copied gate logs.
- d1 fontsys4-r2-original-on-20261022d1 clean/ref0 date2032-01-10: four phases each2890exact/0diff/0refusal/0fallback/0resync/1893positive/997zero/natural0; native route23/23 each; full saves2026-03-02(11765ms),2031-06-02(11563ms) ok1. Fixture copied to MAIN/build-runs/fixtures/<label>.sc4; metrics WT/build-runs/<label>-summary.json.
- e1 fontsys4-r2-original-off-20261022e1 clean/ref0 date2032-01-04; probe OFF (no FontSys lines, no 0/0 equivalence credit), two full saves ok1; capture/metrics at matching fixture/summary paths.
- f1 fontsys4-r2-original-on-reload-20261022f1 clean/ref0 date2032-01-05: staged d1 fixture SHA5ca51d83fbd2cb739a0a445915fc13bb09dd0b75b3d88cfbb92bb121eec6e994; loads saved2031-06-02, full saves both2031-06-03 ok1. Four phases each2890exact/0diff/0refusal, route23/23; fallback/resync0/natural0.
- g1 fontsys4-r2-house40-on-20261022g1 clean/ref0 date2032-01-01: four phases each2890exact/0diff/0refusal, route23/23; fallback/resync0/natural0; full saves2026-03-01(12234ms)/2031-06-01(11703ms) ok1, captured fixture. Ports enabled, terrain/dispatch/effects OFF confirmed.
- h1 fontsys4-r2-house40-off-20261022h1 clean/ref0 date2032-01-01; probeOFF/no-equivalence-credit, full saves2026-03-01(11125ms)/2031-06-01(11156ms) ok1, captured fixture. Profile40 includes simgrids/ports, excludes terrain/dispatch/effects.
- i1 fontsys4-r2-house40-on-reload-20261022i1 clean/ref0 date2032-01-01: staged g1 fixture SHAef224fb8a87d4d5d13b8831004cad85e46df1f3c55c6fcf6d2276aeb3221fa55, full saves both2031-06-02(11750/11609ms) ok1; four phases each2890exact/0diff/0refusal; route23/23, fallback/resync0/natural0.
- New corrected-provider ON totals so far final: 16 sequences,46240exact/0diff/0refusal/0fallback/0resync/natural0;368 native-route checks separate. Historical invalid counts excluded.
- j1 fontsys4-r2-house40-off-reload-20261022j1 clean/ref0 date2032-01-01; staged h1 fixture SHA2d44a05076331519cbda3655ecb4cb0c9e815a7ed52c46b641e5ad5388b43a49; full saves both2031-06-02(10906/10875ms) ok1; probeOFF/no-equivalence-credit; complete shutdown.
- Next: original OFF reload k1 of e1 fixture; run build-runs/fontsys4-audit-evidence.py, final report/queue. No new questions.
- Required controls: exact HOUSE40 profile from current task, ON/OFF full save/shutdown and supported reload to2032/ref0; no experimental keys.
- Request labels are unique; wait via build-runs/fontsys4-wait-verdict.py (never qwait). Helper fontsys4-submit.py fixes HOUSE40 explicitly, rejects reused labels/outstanding requests and checks frozen hashes.
- No new questions or code edits yet. Preserve all historical evidence below until final report.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-fontsys4-r2-manifest.json.

## Prior round report (historical; native-attempt cap wording superseded above)

WIP, no owner verification or replacement credit. The owned original route is safe in the real game, and a bounded catalogue/query/directory/renderer comparison exercised production Rust. Acquisition-dependent comparison evidence is invalidated by a verification-provider defect, corrected after exhausting the native attempt cap. This is not an exact FontSys shadow.

## Implementation

- Scope: drawing-fonts sections 1/12/12.1, actual production `GzFontSys`, `FontFixedInfo`, `FontInfoRecord`. No core algorithm changes, private verification clone, delegated selection, native-query answer injection, or periodic model resync. cGZFont FN18-FN22 excluded.
- Opt-in `fontsys_verify = 1`, default off. Observer load/presave/shutdown hooks run constructed owned instances; natural instrumentation count is explicitly zero. Each comparison retains a production Rust object through its command sequence, then destroys that pair.
- Original route: GZCOM creates CLSID 0x20673960 / service IID 0x287FB697, primary QI 0x00673960, primary vtable checked against published 0x00AC5488 relocated from module base. QI views differ by four bytes. Acquired views are balanced with audited created-reference releases. No global FontSys getter or global-service Shutdown.
- Explicit published slots 9/10/11/13/14 avoid relying on MSVC overloaded virtual declaration ordering. Font getters use published slots 7/10 where necessary. No guessed private getters or shared ABI/checker changes.
- A single independent raw catalogue snapshot per constructed pair provides metadata for 344 read-only Windows fonts. Production Rust populates its own catalogue and computes queries. Corrected adapter additionally captures raw font-header/table-directory bytes once at construction; subsequent content probes use immutable bytes. Provider metrics/raster behavior is outside this FontSys measurement domain.
- Added three verification-only C exports (`fc4_fontsys_verify_new`, `fc4_fontsys_verify_call`, `fc4_fontsys_verify_free`). ABI flag true; ABI version, public headers/tables and defaults unchanged. No binaries committed.
- Main work commits: d916e22d owned-route probe; 7c84f857 production bridge (before call 30); 5067dab6 explicit overloaded slots; bc7d8149 directory/renderer coverage; 8ff837d9 raw-header provider correction and regression.

## Measured game evidence

All requests used unique labels; all completed verdicts were retained. Original attempt cap is exhausted at 3/3. No request is outstanding.

1. `fontsys4-route-20261022a1`: clean/ref_warnings=0, 2035-01-03. Load and shutdown each route exact=23, diff=0, refusal=0, fallback=0, resync=0, natural=0. Initial request placed end_date under ini, so the seconds limit ended it after 2032; not relabeled as a fixed-date control.
2. `fontsys4-focused-20261022b1`: exited -1 before production results during bootstrap AddFont through an overloaded SDK call. No reported foreign exception, Rust panic or reference warning. Corrected call shape to explicit published slots; do not count this as passing conformance.
3. `fontsys4-stable-20261022c1`: clean/ref_warnings=0, no foreign exceptions/stalls/RTC/panics/structured exceptions/other warnings, date 2032-01-05. Full save requested for 2026-03-01 ran on 2026-03-15, `ok=1`, 11641 ms. Each load/presave/shutdown phase: production exact=2823, diff=24, refusal=43, fallback=0, resync=0, natural=0, metadata_once=344, positive=1869, zero=978. Each phase also has 23/23 native route checks, counted separately.

Production totals: 8670 operations = 8469 exact + 72 differences + 129 refusals; fallback=0, resync=0; natural=0 (not exercised). These are constructed raw observations, not an owner-equivalence percentage. Catalogue insertion/order/record queries, availability and directory/renderer cases are nonvacuous. No successful native-versus-Rust acquisition/cache/range/family/codepage/lifetime equivalence claim.

The immutable tested DLL and PDB remain in `build-runs/dll/fontsys4-stable/`; DLL SHA256 `A7D4536F5F3A775EC9818BAB3A697000532E95951058D144D2815F8853B3FC48`. This binary predates the provider correction. Logs/verdicts are in `build-runs/game-results/` under the labels above.

## Failure diagnosis and limits

The old provider returned empty `read_head` data. Production `GzFont::init` correctly refused that content probe even though native opened the real font (first example `C:\WINDOWS\Fonts\AGENCYB.TTF`); downstream handle-dependent comparisons refused. Treat those differences as an invalid provider setup, not a discovered native algorithm or spec discrepancy. Correction 8ff837d9 snapshots real read-only headers and has a Windows integration test proving actual production Arial acquisition, family-cache reuse, codepage propagation and reference/cache release. It has no real-game equivalence verdict.

No fourth original attempt or normal-speed comparison was made. Original/own/current40 immutable control set and saved-fixture reload are incomplete; the successful 2032 save/shutdown run alone does not establish them. No current40 preset or reload script action was found in the allowed operational docs/engine. No promotion, default change, whole-owner coverage credit, or claim about untested paths.

## Gates and questions

- Initial baseline: Rust tests/clippy passed, ctest 68/68. Its native build already included the default-off route probe.
- Stable pre-correction full gate: qbuild all PASS, 4961 Rust tests, clippy zero diagnostics, ctest 68/68. Summary `build-runs/fontsys4-finalgate-summary.txt`.
- Corrected-provider quick gate: PASS, capi 531 tests / 0 failed / 0 ignored and native build PASS; summary `build-runs/fontsys4-provider-quick-summary.txt`.
- Corrected-provider final full gate: qbuild all PASS, 4962 Rust tests / 0 failed / 0 ignored, clippy zero diagnostics, ctest 68/68 (94.2 seconds), native build PASS. Summary `build-runs/fontsys4-corrected-finalgate-summary.txt`. Wrapper notes tree changed during the run: only this lane report changed; engine sources remained committed at 8ff837d9 (confirmed by final diff). This is not a fresh native conformance verdict.
- Corrected built DLL SHA256 `9C7A5FD37144FAC68BD2E5557B01B11B2A88746FA814069D5E2CD558D81881B2`; not game-tested. Preserve the distinct frozen pre-correction DLL and its evidence.
- New Rust module passes qbuild fmt. Root lib.rs traversal reports 645 existing formatting diff lines in untouched children; scoped root check also identifies pre-existing test-module ordering. No unrelated reformat applied.
- Questions: none. The identified failures are adapter defects, not missing native contracts; no speculative question filed. FN18-FN22 remain excluded.

## Next steps

Coordinator review only: preserve the capped evidence and decide disposition of this default-off WIP probe. Corrected-provider native equivalence, independent original/current40 controls and supported saved-fixture reload remain unproven. This lane does not request or authorize another original attempt, restart, merge, push, RE or owner promotion.
