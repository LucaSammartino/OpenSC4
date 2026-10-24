| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-simgrid-label6: shadow summary labels | Codex, `glue-simgrid-label6` | ready; gate PASS; simgrid 73/73, lotrec 73/73 | Coordinator review, credit coverage, land and update regress profile if needed. |

## Changes

Implementation `468673212` changes the simgrid and lotrec summary formats to lowercase `simgrid shadow YYYY-MM-DD <phase>:` and `lotrec shadow YYYY-MM-DD <phase>:` with final `; mismatches=N`. Existing record fields and diagnostic lines remain; phase is in the prefix and julian remains a field. Both use `fc4_date_from_julian`, with the borrowed city simulator's date on load, following the Lua shadow precedent. Shadows remain default-off. No C ABI exports changed; no questions filed.

## Validation

- Stable-source `qbuild.py --wt <WT> all`: PASS. Workspace test totals across all result lines: 6901 passed, 0 failed, 0 ignored; clippy `-D warnings`: 0 diagnostics.
- The same all operation queued coordinator native verification as `qbuild-20261024-035652-build-ctest`: build PASS 240/240 edges, CTest PASS 74/74, terminal `fc4verify: PASS` and `qbuild: PASS`. No direct MSVC configure and no duplicate verify operation. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- Quick Python regex checks used the documented coverage pattern without opening tools/. Both source format samples and every real summary line matched. Helper/evidence: `build-runs/check-shadow-labels.py`.
- [Run 400](../runs/glue-simgrid-label6-2032-20261024a-403.md), request `glue-simgrid-label6-2032-20261024a`: Konradshohe turbo through 2032, both ini shadow keys enabled; verdict **clean**, `ref_warnings=0`, all reported problem counters zero, shutdown observed. DLL SHA256 prefix `8f04c046`, slot 3, wall 161 s.

Exact clean/total lines:

```text
simgrid 73/73
lotrec 73/73
```

Each has one load and 72 monthly summaries, all `mismatches=0`. Load date is `2026-01-27`, simgrid `exact=86/86`, lotrec `exact=741/741`. Last month is `2032-01-01`, simgrid `exact=86/86`, lotrec `exact=672/672`. The clean autotest verdict reached `2032-01-11`. Evidence: `build-runs/game-results/glue-simgrid-label6-2032-20261024a.{log,verdict}`. Harness summary independently reports 73/73 clean for each label, zero resynced lines, zero incomplete lines.

## Next steps

Coordinator can credit the measured shadows and land this commit. tools/ stayed unread and unchanged under the clean-room rule, including coverage tables and regress.ps1. If needed, coordinator should append these exact shadows to the current single regress shadows-profile string after review; no main merge or tools edit was attempted in this detached lane. No implementation work remains.
