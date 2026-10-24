| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lua-replay6: bounded Lua registration shadow | Codex; glue-lua-replay6 (detached worktree) | in progress | Implement LH6/LH7 bounded comparison, native gate, unique 2032 game run. |

## Handoff

- Goal: extend bounded registration comparison per lua-host LH6/LH7; emit monthly `lua shadow` summaries.
- Prior lane: glue-lua-shadow5 measured lua-registry 73/73; full execution replay is unsupported by public observation.
- Done: LH6/LH7 §§1.4.2/3.5/8.6 read; full replay remains disabled. Native capture failures and image-validated portable evidence labels added; requested `lua shadow` summary label with explicit registration scope.
- Model checks scratch at each registration stage; 6 focused capi tests passed (ordering, missing bindings, exact actual identities included).
- Source milestone: `c9a62c9b1`; owned Rust fmt and diff check clean.
- Final-source qbuild all PASS (collected to exit 0): workspace 6890 passed, 0 failed, 0 ignored; clippy 0 diagnostics; native fc4verify build 233/233 (115.1 s), CTest 72/72 (146.2 s), verify PASS 265.1 s. Native ID `qbuild-20261024-030121-build-ctest` complete.
- Gate noted tree changes: documentation milestone commit and README scope clarification only; runtime source stayed c9a62c9b1 throughout. No duplicate checks.
- Next: submit and collect unique `lua-replay6-c9a62c9b1-turbo-2032-20261024a` run (no qwait); record run >=382 in label-only filename. Then final report, merge main as task explicitly requests, queue full actual HEAD.
- Formatting first used a relative path rejected by qbuild; corrected to absolute owned-file path. No source compilation failure.
- Task explicitly overrides spec §8.6 evidence label `lua-registry`; `lua` summary must still confer no full-VM verification credit.
- No game/native requests outstanding at this gate milestone. Prepared watcher: build-runs/wait-lua-verdict.ps1.
- Do not access tools/ or analysis sources. No ABI export changes intended.
