| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-legacyctl-shadow12 (legacy control shadow) | `Codex` / `glue-legacyctl-shadow12` (engine lane) | in progress | Inspect observable boundary contracts; implement and validate supported shadow. |

## Handoff
- Goal: default-off same-process legacy control state shadow, measured through 2032.
- Done: 971d183e3 owner-only Rust replay/private transport and default-off vtable observers committed.
- Coverage: primary QI/AddRef/SetOwnControl, legacy GetControl/SetOwnControl; all five owner fields compared.
- SetParameters counts and post-build GetControl probes are separate; property/interpreter replay excluded.
- Questions: LC14 return contracts; LC15 observable direct-call boundary, in docs/specs/questions/legacy-control.md.
- Next: ONE game request OUTSTANDING: legacyctl12-owner-2032-971d183e3-r1 (turbo to 2032). Wait with build-runs/wait-legacyctl-verdict.ps1 on build-runs/game-results/legacyctl12-owner-2032-971d183e3-r1.verdict. Never discard an existing verdict or duplicate request. Then count legacyctl shadow exact/total and create run record/finish report/queue.
- Final all session 43105 TERMINAL PASS: cargo 7375/0, clippy 0 diagnostics, native build 159/159 edges, CTest 81/81. Queue qbuild-20261024-184451-build-ctest; source fix 021be6fe6; log build-runs/legacyctl-final-all-console.log. No further build required without source changes.
- Focused result: terminal, 2 passed/0 failed (4870 filtered), 55m41s compile; log build-runs/legacyctl-focused.log. PowerShell wrapper exit 1 reflects normal cargo stderr NativeCommandError, not a test failure.
- First all: PASS, cargo 7375/0, clippy 0 diagnostics, native build 263/263 edges and CTest 81/81. Queue qbuild-20261024-182826-build-ctest; all delegates native verification correctly, do not separately enqueue verify.
- Evidence before review fix: build-runs/legacyctl-pre-qi-fix/; final source needs a new all check because C++ changed.
- Review fix applied: QI out-pointer representation stays in unsigned-char bytes and is compared bytewise for both success/rejection; no indeterminate pointer/integer value is evaluated.
- Game draft: build-runs/legacyctl-owner-2032.req-draft; wait helper build-runs/wait-legacyctl-verdict.ps1 uses one file event, no qwait/poll loop.
- Focused check: build-runs/legacyctl-focused.log; no game request outstanding.
- Formatting: PASS, three explicit owned Rust files. Focused compilation is unusually slow but active (CPU increased, one rustc completed).
- Initial relative-path fmt and q.py Bash permission failure preserved in build-runs; no source diagnostics yet.
- Constraints: no native signatures inferred; no prohibited sources; no agents.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-legacyctl-shadow12-r1-manifest.json.
