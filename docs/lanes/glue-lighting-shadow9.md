| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lighting-shadow9: lighting state shadow | Codex, `glue-lighting-shadow9` | in progress | Inspect public contracts and implement monthly comparison. |

## Handoff
- Goal: default-off lighting_shadow, independent Rust expectations, house-safe monthly native comparison through 2032.
- Done: initial handoff a17dafae8; implementation 9e34944a1 adds default-off read-only native slot 14/16(null) comparison of 14 scalar fields to independent shipped-parameter Rust core; monthly deduplication including load month.
- Focused tests: 2/2 lighting_shadow tests pass; explicit-file fmt passes. Initial relative fmt path and Bash q wrapper failed (no source failure); absolute fmt and direct cargo succeeded.
- Open: LT14 in docs/specs/questions/lighting.md requests ordered external-input replay; whole-owner coverage not claimed. Slots 0–13, 15, 17–29 and nonnull-holder slot 16 uncompared.
- Gate PASS: stable-source qbuild all completed, workspace tests 6986/6986, clippy 0 diagnostics, queued native build 254/254 edges, full CTest 79/79. Native job qbuild-20261024-083722-build-ctest and session 16856 collected to terminal PASS; do not repeat.
- Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log. Focused tests 2/2; absolute-file fmt PASS.
- Game r1 COMPLETE: lighting9-house-2032-r1-20261024 clean, foreign_exceptions/ref_warnings/rtc=0, date 2029-10-22 at 601.828 s. lighting9-house-2032-r1-20261024 0/46; each line one texture mismatch (69D3A23B native vs 69D3BC3B literal table), all other 13 fields exact. Record docs/runs/lighting9-house-2032-r1-20261024-469.md; LT15 filed. No game request outstanding.
- Diagnostic revision: capture all six §5.6 properties from independent public resource 6534284A/A9189CF0/00000001 once at load; feed Parameters::initial().read(inputs), never lighting outputs. Focused tests now 3/3.
- Final gate r2 PASS on source fad1dbe42: Rust workspace 6987/6987, clippy 0 diagnostics, native build 207/207 edges, CTest 79/79. Queued qbuild-20261024-090555-build-ctest and exec session 8711 collected to terminal PASS. Do not repeat.
- Game r2 OUTSTANDING: lighting9-house-2032-r2-20261024, seconds=1500, timeout=1800, exact house switches plus lighting_shadow=1. Wait on build-runs/game-results/lighting9-house-2032-r2-20261024.verdict with wait-lighting-verdict.py; if its 540-second bound expires, wait AGAIN on SAME label, never resubmit. No other request outstanding.
- First r2 bounded wait (exec 18857) completed with timeout, not a game verdict. Continue waiting on that same result file. The r2 time limit intentionally exceeds one wait window.
- Next: collect r2 final verdict/counts; record free run number 470 (recheck local docs/runs), update full report and READY queue for measured work. Preserve r1 evidence. Full owner remains blocked by LT14.
- Prepared request: exact house ini switches (Compare-Object verified), plus lighting_shadow=1, turbo to 2032-01-01. DLL engine/build-x86/FreeCity4.dll.
- Constraints: no prohibited source or tools; no agents; no branch changes; shared files additive.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-lighting-shadow9-r1-manifest.json
