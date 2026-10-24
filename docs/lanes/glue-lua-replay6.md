| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lua-replay6: bounded Lua registration shadow | Codex; glue-lua-replay6 (detached worktree) | in progress | Implement LH6/LH7 bounded comparison, native gate, unique 2032 game run. |

## Handoff

- Goal: extend bounded registration comparison per lua-host LH6/LH7; emit monthly `lua shadow` summaries.
- Prior lane: glue-lua-shadow5 measured lua-registry 73/73; full execution replay is unsupported by public observation.
- Done: LH6/LH7 §§1.4.2/3.5/8.6 read; full replay remains disabled. Native capture failures and image-validated portable evidence labels added; requested `lua shadow` summary label with explicit registration scope.
- Model checks scratch at each registration stage; 6 focused capi tests passed (ordering, missing bindings, exact actual identities included).
- Source milestone: `c9a62c9b1`; owned Rust fmt and diff check clean.
- Final-source qbuild all: workspace 6890 passed, 0 failed, 0 ignored; clippy 0 diagnostics. Same operation still running, session 63325, queued native ID `qbuild-20261024-030121-build-ctest`.
- Next: collect that native gate to terminal result, then unique 2032 run (no qwait).
- Formatting first used a relative path rejected by qbuild; corrected to absolute owned-file path. No source compilation failure.
- Task explicitly overrides spec §8.6 evidence label `lua-registry`; `lua` summary must still confer no full-VM verification credit.
- No game requests outstanding. Do not duplicate the native gate request above.
- Do not access tools/ or analysis sources. No ABI export changes intended.
