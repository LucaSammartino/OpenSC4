| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-legacyctl-shadow12 (legacy control shadow) | `Codex` / `glue-legacyctl-shadow12` (engine lane) | in progress | Inspect observable boundary contracts; implement and validate supported shadow. |

## Handoff
- Goal: default-off same-process legacy control state shadow, measured through 2032.
- Done: 971d183e3 owner-only Rust replay/private transport and default-off vtable observers committed.
- Coverage: primary QI/AddRef/SetOwnControl, legacy GetControl/SetOwnControl; all five owner fields compared.
- SetParameters counts and post-build GetControl probes are separate; property/interpreter replay excluded.
- Questions: LC14 return contracts; LC15 observable direct-call boundary, in docs/specs/questions/legacy-control.md.
- Next: collect REQUIRED qbuild all session 7674, already launched on stable source; never duplicate it.
- Focused result: terminal, 2 passed/0 failed (4870 filtered), 55m41s compile; log build-runs/legacyctl-focused.log. PowerShell wrapper exit 1 reflects normal cargo stderr NativeCommandError, not a test failure.
- Full gate Rust results: 7375 passed/0 failed; clippy clean (Finished dev profile, 1m09s). Session 7674 still active awaiting remaining/native result; no terminal verdict yet. Logs: build-runs/qbuild-test.log, qbuild-clippy.log, eventual legacyctl-all-console.log.
- Native verify: use qbuild verify only if all does not already delegate it; then game to 2032.
- Review finding pending fix AFTER session 7674 terminal: QI pre-out may be uninitialized. Current uint32 memcpy plus reinterpret_cast still evaluates those bits. Keep pre-out as unsigned-char bytes and compare by memcmp on rejection; decode native output only on native success. Preserve current gate logs and rerun all after the C++-only fix. Do not game-test before fixing.
- Game draft: build-runs/legacyctl-owner-2032.req-draft; wait helper build-runs/wait-legacyctl-verdict.ps1 uses one file event, no qwait/poll loop.
- Focused check: build-runs/legacyctl-focused.log; no game request outstanding.
- Formatting: PASS, three explicit owned Rust files. Focused compilation is unusually slow but active (CPU increased, one rustc completed).
- Initial relative-path fmt and q.py Bash permission failure preserved in build-runs; no source diagnostics yet.
- Constraints: no native signatures inferred; no prohibited sources; no agents.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-legacyctl-shadow12-r1-manifest.json.
