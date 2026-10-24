| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lua-replay6: bounded Lua registration shadow | Codex; glue-lua-replay6 (detached worktree) | in progress | Implement LH6/LH7 bounded comparison, native gate, unique 2032 game run. |

## Handoff

- Goal: extend bounded registration comparison per lua-host LH6/LH7; emit monthly `lua shadow` summaries.
- Prior lane: glue-lua-shadow5 measured lua-registry 73/73; full execution replay is unsupported by public observation.
- Done: LH6/LH7 §§1.4.2/3.5/8.6 read; full replay remains disabled. Native capture failures and image-validated portable evidence labels added; requested `lua shadow` summary label with explicit registration scope.
- Model checks scratch at each registration stage; 6 focused capi tests passed (ordering, missing bindings, exact actual identities included).
- Next: owned Rust formatting, commit source milestone, then one final qbuild all (native verify) and unique 2032 run.
- Formatting first used a relative path rejected by qbuild; corrected to absolute owned-file path. No source compilation failure.
- Task explicitly overrides spec §8.6 evidence label `lua-registry`; `lua` summary must still confer no full-VM verification credit.
- No game/native requests outstanding; no source changes yet.
- Do not access tools/ or analysis sources. No ABI export changes intended.
