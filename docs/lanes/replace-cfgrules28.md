| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-cfgrules28 graphics rules (engine) | Codex / .claude/worktrees/replace-cfgrules28 / detached HEAD; reserved branch replace-cfgrules28 | blocked; source commit 3116a23da; private transport only; ABI exports unchanged; Rust 7877 passed, clippy clean, CTest 120/120; game/shadow evidence unmeasured; GR9 | 1. Resolve GR9. 2. Bind/register and trace cfgrules. 3. Measure acceptance and enqueue ready. |

- Implemented private static-link transport over the production core for Init, SetOptions, explicit logging, ApplyOptionGroupSetting and GetOptionGroupAutoSetting; mutex-protected getters and retained renderer references.
- Native acquisition is required explicitly: unavailable measurements refuse transport initialization before replacing the renderer or parsing; no guessed FSAA/backing-store values and no original-manager output callback.
- Native GZCOM class, default-off `cfgrules` registration and query tracing remain missing. **cfgrules is not ready for the replace list.**
- Full gate: qbuild `all` PASS on source commit 3116a23da; Rust 7877 passed/0 failed/0 ignored ([log](../../build-runs/qbuild-test.log)); clippy 0 diagnostics ([log](../../build-runs/qbuild-clippy.log)); native build 82/82 edges ([log](../../build-runs/fc4verify-build.log)); CTest 120/120 ([log](../../build-runs/fc4verify-ctest.log)). Owned Rust file fmt PASS.
- Boundary test: `cfgrules_owner_test` PASS, 0.02 s, including typed ordered duplicate writes, balanced retains/releases, same-renderer Init, unavailable acquisition, logging bytes/preferences and concurrent getters ([output](../../engine/build-x86/Testing/Temporary/LastTest.log:784)).
- Query comparison, key off/on: not run; native binding blocked by GR9. No measured query parity.
- Acceptance (a), house replace list plus cfgrules through 2032: not run; verdict/ref_warnings unmeasured.
- Acceptance (b), golden Konradshohe through 2026-04-01, plain and generated action script: neither run; exact/envelope result unmeasured.
- Acceptance (c), regress shadows profile with cfgrules: not run; shadow invariance unmeasured.
- Question GR9: missing interface-only GPU probe recipe, backing-store-presence acquisition and remaining version/sound provider details ([question](../specs/questions/graphics-rules.md:36)); related renderer specs did not close the probe/presence gap. Existing parser/profile questions remain open.
- Scope incident: an overbroad search matched archive filenames and two lines of `docs/specs/questions/answered/graphics-rules.md` after the source commit. Those facts were already in the published spec; no source edits followed. The archive was not used as a behavior source.
- Landing: hold a `wip` merge-queue entry; no ready/train enqueue or credit promotion.
- Next: publish GR9 acquisition contract, implement the FC4 class/default-off key and full query tracing, measure (a)-(c), rerun the gate after code changes, then enqueue ready after the supervisor publishes the reserved branch.
