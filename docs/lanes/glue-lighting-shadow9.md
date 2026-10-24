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
- Game request OUTSTANDING: lighting9-house-2032-r1-20261024 (build-runs/game-queue, verdict under game-results). One blocking wait-lighting-verdict.py operation is active as exec session 28885; collect that SAME operation to terminal result. Do not submit another request.
- Next: collect game verdict and 14-field lighting counts, file any observed differences as questions, write run record (next free local run number 469, recheck before writing), then finish measured report/READY queue.
- Prepared request: exact house ini switches (Compare-Object verified), plus lighting_shadow=1, turbo to 2032-01-01. DLL engine/build-x86/FreeCity4.dll.
- Constraints: no prohibited source or tools; no agents; no branch changes; shared files additive.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-lighting-shadow9-r1-manifest.json
