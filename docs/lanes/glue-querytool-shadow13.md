| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-querytool-shadow13: query tool predicate shadow | `Codex` / `glue-querytool-shadow13` (engine lane) | READY, partial: cargo 7407/0, clippy clean, CTest 81/81; run 631 clean through 2032-01-15, ref_warnings=0; querytool 73/73, mismatches=0; CT27 open; private C ABI adapter added | 1. Resolve CT27 for full target/display/highlight observation. 2. Coordinator review and append the key to the current shadows profile. |

## Report

Added default-off `querytool_shadow = 1` observation at city load and each month. The fixture walks live city occupants through occupant-manager slot 44, copies their raw type and presence of properties `0x8A416A99` and `0xCAA9AB92`, evaluates the native hover/describe filters, and compares both outputs with the production core's `query_tool::accepts` predicates. Core inputs contain no native predicate answers. A focused adapter test covers false-valued but present properties.

Native stack filter records follow occupant-manager sections 13.2.1/13.2.6 and city-tools-input sections 8.8/8.12. The compat accessor requires the identified supported image and validates table addresses. The walk uses the null filter specified by occupant-manager sections 9.2/10. Occupant pointers remain borrowed from callbacks; stack filters are used only for predicate calls. Empty samples and exceptions count as mismatches.

Scope is **hover/describe predicates only**. Every line states `partial=1` and excludes renderer hits, selected targets, displayed fields, strings and highlights. Full `cSC4ViewInputControlQuery` verification remains open under CT27.

Milestones: `e96cbfbba` adds the shadow/config/observer/private adapter; `b558f8c07` fixes the declared-only SDK concrete filter linker failure; `e52481945` corrects the contract citation. The private `fc4_querytool_shadow_pick` C symbol is additive; public headers and `FC4_ABI_VERSION` are unchanged.

## Gate

Final `qbuild.py all` **PASS**: workspace cargo **7407 passed, 0 failed, 0 ignored** (sum of all test-result lines), Clippy exit 0 / diagnostics 0 with warnings denied, CMake **47/47 edges**, **CTest 81/81**, 0 failed (273.7 s). Native queue: `qbuild-20261024-202138-build-ctest`. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`; queue verdict under `build-runs/build-results/`.

The native verdict's tree-changed annotation reflects the reference-citation/lane commit during the queued operation. C++ bytes were stable before native compilation; source did not change after that build. The earlier `qbuild-20261024-195550-build-ctest` failed to link SDK concrete filter methods and skipped CTest. Its evidence is preserved in `build-runs/failed-sdk-filter-link/`. The previous transport's inaccessible gate remains separately incomplete in `build-runs/incomplete-previous-round/`.

## Real city

[Run 631](../runs/querytool13-r2-20261024a-642.md), request **querytool13-r2-20261024a**, Konradshohe, turbo, `enable=simulator`, `querytool_shadow=1`, requested end date 2032-01-01:

```text
querytool 73/73
```

All 73 load/month observations exact, **mismatches=0**: 502193 occupant observations and 1004386 predicate comparisons. Per sample: 6771–7065 occupants, 13542–14130 comparisons. Native accepted totals: hover 396808, describe 396516. Totals count repeated observations across months.

Verdict **clean**, final date **2032-01-15**, real_seconds=28.891, days_per_second=75.45; foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, **ref_warnings=0**; Rust panics=0, structured exceptions=0, shadow INCOMPLETE lines=0. DLL SHA256 prefix `c5cff5d8`. Turbo had no differences requiring normal-speed replay. No requests remain outstanding.

Evidence: `build-runs/game-results/querytool13-r2-20261024a.verdict` and `.log`; `build-runs/summarize-querytool.py` produces the count above. The file-event wait helper `build-runs/wait-querytool-game.ps1` was collected to completion in the same operation, without qwait.

## Questions and next steps

**CT27**, `docs/specs/questions/city-tools-input.md`, requests the deterministic nonmodal renderer-hit/output fixture contract. Full target, display/string and highlight observation awaits that contract. Existing CT13/CT14/CT20 were referenced rather than duplicated.

The key remains default-off. This clean-room lane did not access or edit `tools/regress.ps1` or the coverage maps, and did not merge main. The coordinator can review the partial scope, append `querytool_shadow` to the current single shadows-profile string, and assign only the coverage this fixture establishes.
