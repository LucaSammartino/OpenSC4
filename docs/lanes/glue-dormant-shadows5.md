| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dormant-shadows5 (six dormant record shadows) | Codex / detached `glue-dormant-shadows5` | wip: full gate PASS (cargo6648/0, clippy0, CTest69/69); combined native2032 clean/ref0; four exact keys; propmgr219/220 and lotcfg292/293 retain PL23 | Coordinator adds four exact profile keys; resolve PL23 before prop/lot promotion |

## Result

All six requested shadows ran together against native owners in Konradshohe through **2032-01-01**.
The run was **clean**, `ref_warnings=0`, with all exception, stall, RTC and overrun counters zero.
Four keys are exact in their implemented comparison subsets. Two retain the already-filed **PL23** differences.
No core/plugin source, ABI exports, default settings, coverage maps or effects shadow changed.

Evidence commit: `435e2864`. Full run record: [dormant5-all-2032-20261023-r1a](../runs/dormant5-all-2032-20261023-r1a.md).
DLL SHA256: `e1944d66d824d1178f3f64b3bd7ca4c1f5cb5c60a4568ae612c3b3668b8d362d`.
Ignored request, complete log and verdict are retained in this worktree's `build-runs/game-queue/done/`
and `build-runs/game-results/dormant5-all-2032-20261023-r1a.*`. No request remains outstanding.

## Exact counts and owner mapping

The following are clean/total **shadow lines** as reported by the verdict:

```text
citydetail 1/1
propmgr 219/220
lotcfg 292/293
tutorial 146/146
```

The variant/feature implementations emit **check** counters instead of `mismatches=N` shadow lines;
they are absent from the harness's generic shadow-line summary. Their direct logged totals normalize to:

```text
variant 420/420
feature 13172/13172
feature lifetime 15/15
```

| Key | Native owner compared | Scope and measured evidence |
|---|---|---|
| `propmgr_shadow` | `cSC4PropManager` | Native city manager record roundtrip; process catalogue, ordered live city/lot/street queries and uninserted prop creation. 219/220 lines clean; one catalogue line has1544 differences. Native-record bootstrap is disclosed and earns no independent-evolution/algorithmic credit. |
| `propmgr_shadow_cells=32` | Same `cSC4PropManager` | Sample-count parameter for the preceding shadow, not an additional owner or independent shadow. |
| `lotcfg_shadow` | `cSC4LotConfigurationManager`; resource configuration arrays/getters involve `cSC4LotConfiguration` | Native manager record, map/families, ordered ID/HasBuilding/object queries, and resource object arrays at current detail. 292/293 lines clean; one array line has1908 differences. Configuration loading/getter checks include C++ glue and the Rust object decoder; no whole-configuration-owner credit. |
| `tutorial_shadow` | `cSC4TutorialSystem` | Record plus IsTutorial, task number and GUID; 146/146 lines clean. Window/registry/cheat behavior excluded. |
| `variant_shadow` | Native standalone variant storage through `cIGZVariant` (the `cRZVariant` method family); clone source is the embedded `cSCProperty` variant interface | 70 literal scalar/array cases,420/420 checks, failures0/refusals0,71 native clones. Published embedded table00AA6788 and standalone table00ACECD0 have distinct identity/lifetime routes. Comparison mutates owned standalone copies, not the borrowed property. Production cFC4Variant calls Rust GetCount196 times;70 typed-set and70 core-write calls;70 production clones, no clone refusal/fallback/resync. No whole127-slot interface, property-owner, conversion, string/object/pointer/reference-alias or native-serialization equivalence. |
| `citydetail_shadow` | Fresh native `cSC4CityDetailManager` factory objects against Rust city-detail core | One load comparison: fresh/read-same/read-none/read-v1 and record bytes, mismatches0. Setting1. Native and ours both rejectv1. Ours-only dry-run level2/3 removal passes are explicitly excluded from comparison credit. |
| `citydetail_seed=1592594157` | Replacement `cSC4CityDetailManager` configuration parameter | Accepted in the combined ini; no replacement enabled. The shadow dry runs use their existing private fixed seed0x5EED0EAD, not this setting. No separate comparison/owner credit for the seed. |
| `feature_shadow` | Application-owned `cSC4FeatureManager` via borrowed app getter plus owned QI hold | Independent captured32-resource Rust snapshot; feature IDs, all32 pack-shift residues at unsigned boundaries, highest pack, and bounded QI/AddRef/Release.74 samples,13172/13172 query checks;15/15 lifetime checks; diff0/refusal0/resync0, final app hold1. No native Init/Shutdown/static reset/release-to-zero or full-owner lifetime claim. |

## Remaining differences and profile handoff

Add these exact keys to the coordinator-owned shadows profile:
`tutorial_shadow=1`, `variant_shadow=1`, `citydetail_shadow=1`, `feature_shadow=1`.
The task's clean-room rule prohibits opening/reading any part of `tools/`; consequently this lane leaves
`tools/regress.ps1` edits to the coordinator. Neither coverage map was edited. No profile promotion is claimed
until that integration is applied. Existing keys remain opt-in/default-off.

Leave **`propmgr_shadow` and its `propmgr_shadow_cells` parameter, plus `lotcfg_shadow`, out**:

- Prop catalogue:1544/1544 padding-only differences, keysDiffering0, validDiffering0,
  profilesDiffering0, padOnly1544; families and catalogue data2 exact. The three bytes after wealth
  are unwritten native memory; prop-lotconfig A3.2 explicitly chooses zero in ours.
- Lot configuration arrays:1908/1909 configurations differ,25210 objects,1908 appended foundations,
  unbound0. Printed first differences are final records. Lot-configuration3.7 explicitly chooses
  zero for the unwritten foundation object ID. Other array fields have not been exhaustively ruled out.

Appended this run's evidence to **PL23** in `docs/specs/questions/prop-lotconfig.md`; no duplicate ID filed.
No small spec-supported fix exists for these raw differences, so no implementation or comparison filter was
changed. Both differences were also observed at normal speed in the prior verify-propcfg4 run
`vpc4-r2-normal-20261022a`; this is not a turbo-only difference, so no redundant normal-speed run was launched.
The requested wip stop applies pending the missing raw-equivalence contract. No rerun on unchanged source.

## Validation and reviewed history

One final-source `qbuild all` completed: Rust6648 passed/0 failed/0 ignored, clippy0 diagnostics,
native build220/220 edges and full CTest69/69. Its native steps used the coordinator queue
`qbuild-20261023-231210-build-ctest` (the same build/CTest route used by qbuild verify).
No direct MSVC configure/build or duplicate native check was launched. Full logs:
`build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.

The single game request had turbo1, end_date2032-01-01, timeout660, all six keys on, cells32,
citydetail_seed1592594157, and an empty replacement list. It completed in71.375 simulation seconds,
91 seconds wall, with zero incomplete or resynced shadow lines. The event-driven blocking helper
`build-runs/wait-verdict.ps1` supports an already-existing verdict and never used qwait or a polling loop;
the original wait was collected to terminal completion.

Reviewed implementations: FC4PropLotShadow.cpp, FC4VariantShadow.h, FC4CityDetailShadow.cpp,
FC4FeatureShadow.cpp and config/lifecycle call sites. Relevant lane history: glue15,
verify-propcfg4, glue-citydetail4, verify-variants4, core-feature-manager4. No separate feature-plugin lane
report was found by key/content search; plugin history is ae70c86f,8045b3c1,0465e0bf.

Next: coordinator applies four exact profile keys and reviews owner scopes before crediting coverage;
spec writers resolve PL23 before prop/lot raw-shadow promotion. No merge, push or branch/ref update performed.
