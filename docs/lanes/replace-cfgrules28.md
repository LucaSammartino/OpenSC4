| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-cfgrules28 graphics rules (engine) | Codex / .claude/worktrees/replace-cfgrules28 / detached HEAD; reserved branch replace-cfgrules28 | blocked on GR10; source 254e7c309; final gate PASS: Rust 7888, clippy 0, CTest 128/128; ABI unchanged; refcheck clean/0 warnings | Specify native rule-file roots/search; measure query parity and (a)-(c); enqueue ready only after acceptance passes. |

- GR9 implemented: native twenty-draw FSAA/readback, reversible backing-store presence, executable resource/Windows/CPU acquisition and optional sound strings; unavailable acquisition refuses Init.
- Added GZCOM class 0x0A1D8285/IID 0x0A1D8284 over the production Rust owner; default-off `cfgrules`; ABI exports/version unchanged.
- Query/property tracing and default/enabled/veto factory tests added; original tracing uses `compare = class:cSC4ConfigRuleManager` (record mode).
- Final round: no new engine changes; finished the full gate on d94b8d83f (same engine source as corrected 254e7c309).
- `qbuild all` PASS: Rust 7888 passed/0 failed/0 ignored; clippy exit=0/0 diagnostics; WT `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`.
- Native gate PASS: build 62/62 edges; CTest 128/128, 0 failed (87.3 s); WT `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`; qbuild terminal verdict PASS.
- Off/on evidence, source 9e9a10f24: main `build-runs/game-results/cfgrules28-r2-queries-off1.log` and `cfgrules28-r2-queries-on1.log`; verdicts clean/0 warnings and clean/3 warnings respectively.
- Comparison rechecked: 20/77 final renderer properties differ; original traces 12 recommendation groups, replacement getter traces 0 because Rust logging calls internally. Recommendation parity remains unmeasured.
- On1 opens neither `Apps/Graphics Rules.sgr` nor `Plugins/Graphics Options.sgr`; Init status=1 installs fallback. GR10 path/search contract is still absent from the permitted specs; this item remains stopped.
- Corrected source 254e7c309: main `build-runs/game-results/cfgrules28-r2-refcheck2.log` and `.verdict`: clean, foreign_exceptions=0, ref_warnings=0, 2026-04-06; DLL SHA prefix 7ae0f202. Still fallback; no golden claim.
- Acceptance (a), house replace list + cfgrules clean to 2032/ref_warnings=0: unmeasured, blocked by GR10. Earlier combined -Queue/-Parallel invocation was rejected before submission.
- Acceptance (b), golden plain and generated-action comparisons to 2026-04-01: 0/2 measured, blocked by GR10; no exact/within-envelope result.
- Acceptance (c), unchanged regress shadows with cfgrules: 0/40 baseline keys measured, blocked by GR10; no shadow-exact claim.
- Existing question GR10: `docs/specs/questions/graphics-rules.md`; source carries SPEC-GAP. UTF-8 and nearest/even profile limits remain.
- cfgrules is NOT ready for the replace list. Full gate is green; behavioral acceptance is incomplete. WIP merge entry only; no ready/train enqueue or coverage credit.
- Next: obtain GR10; bind specified paths and complete recommendation tracing; repeat off/on and (a)-(c), then gate and enqueue the actual published tip when all acceptance checks pass.
