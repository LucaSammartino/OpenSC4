| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-model-reg-effects5 (model effects owner closure) | Codex / core-model-reg-effects5 | in progress | Audit existing model_reg_effects against graphics-formats section 3.11 and conformance tests |

## Handoff
- Goal: close cSC4ModelRegEffects in core only; no ABI/plugin changes.
- Existing implementation: engine/core/src/model_reg_effects/{mod.rs,tests.rs}; no README yet.
- Next: read section 3.11, tests 48 onward, audit implementation and extend missing behavior.
- Worktree is reserved and detached; commit normally, do not switch refs.
- No game requests outstanding; core-only task has no shadow hook.
