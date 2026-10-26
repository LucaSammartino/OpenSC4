| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-cfgrules28 graphics rules (engine) | Codex / .claude/worktrees/replace-cfgrules28 / detached HEAD; reserved branch replace-cfgrules28 | blocked on GR10; merged main 8e49c9ce3; source 254e7c309; ABI unchanged; native 128/128; full Rust gate pending; final refcheck clean/0 warnings | Resolve native rule-file roots/search; repeat query parity and (a)-(c); complete full gate, then enqueue only on measured success. |

- GR9 implemented: native twenty-draw FSAA/readback, reversible backing-store presence, executable resource/Windows/CPU acquisition and optional sound strings; unavailable acquisition refuses Init.
- Added GZCOM class 0x0A1D8285/IID 0x0A1D8284 over the production Rust owner; default-off `cfgrules`; ABI exports/version unchanged.
- Query/property tracing and default/enabled/veto factory tests added; original tracing uses `compare = class:cSC4ConfigRuleManager` (record mode).
- Current native gate PASS: 44/44 build edges, CTest 128/128 (99.2 s), source 254e7c309; WT `build-runs/fc4verify-build.log` and `fc4verify-ctest.log`.
- Earlier quick PASS: 670 C API tests and native 150/150 edges; acquisition and transport executables PASS; owned Rust fmt PASS. Workspace `qbuild all` still running (session 98353; WT `build-runs/qbuild-test.log`); current clippy not yet run.
- Main game logs/verdicts: `cfgrules28-r2-provider-smoke1` clean/0 warnings but Init refused (old SDK path binding); not acceptance.
- Off/on source 9e9a10f24: `cfgrules28-r2-queries-off1` clean/0 warnings; `cfgrules28-r2-queries-on1` clean/3 warnings; both main `build-runs/game-results/`.
- Query comparison: 20/77 final render properties differ; original logs 12 internal recommendation queries, replacement native getter is not called during Rust logging. Recommendation parity unmeasured.
- On1 opens neither `Apps/Graphics Rules.sgr` nor `Plugins/Graphics Options.sgr`; Init status 1 installs fallback. GR10 asks for exact native path/search acquisition; stopped this item rather than guess folders.
- Corrected source 254e7c309: `cfgrules28-r2-refcheck2` clean, foreign_exceptions=0, ref_warnings=0, date 2026-04-06; DLL SHA prefix 7ae0f202. Still Init status 1; software record (hardware=0), backing=1, sound=1. No golden claim.
- Acceptance (a), house list + cfgrules to 2032: unmeasured; CLI rejected combined -Queue/-Parallel before submitting a game; use -Parallel alone after GR10.
- Acceptance (b), golden plain/generated-action to exactly 2026-04-01: neither run; stopped on GR10.
- Acceptance (c), every shadow key vs main `regress-train267-shadows.log`: not run; 40 baseline keys identified, invariance unmeasured; stopped on GR10.
- Question: GR10 appended to `docs/specs/questions/graphics-rules.md`; source carries SPEC-GAP. Existing UTF-8 and nearest/even profile limits remain.
- cfgrules is not ready for the replace list. No ready/train enqueue or coverage credit; keep merge entry wip.
- Next: finish the running full gate; obtain GR10; bind paths and add candidate recommendation trace; repeat off/on, (a)-(c), full gate, then enqueue the actual published tip.
