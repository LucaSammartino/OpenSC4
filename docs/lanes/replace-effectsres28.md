| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-effectsres28: effects resource (engine) | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/replace-effectsres28; detached HEAD; reserved branch replace-effectsres28 | BLOCKED EF25; code a7c3c61f4; ABI exports unchanged; cargo 7879 passed/0 failed; clippy PASS; CTest 119/119; game not run; shadows unmeasured | 1. Resolve EF25. 2. Implement the resource factory/publication adapter and measure (a)-(c). |

- Fixed the specified root-version rejection (effects sections 3, 18.8.1) in both directory parsers; only versions 3 and 4 are accepted before section decoding.
- Added two regression tests covering supported, unsupported and byte-swapped headers through core and C ABI; both pass in `build-runs/qbuild-test.log`.
- [EF25](../specs/questions/effects.md): the resource primary vtable declarations, serialization/QI sub-object layout and lifecycle contracts needed by the original factory are missing. Native registration stopped rather than inventing ABI facts.
- The existing directory writer explicitly emits simulation sections for fixtures; it does not establish full-resource serialization compatibility.
- (a) House replace list plus effectsres through 2032: **unmeasured**, no run/log; no safely implemented resource class/key, ref_warnings unavailable.
- (b) Cached-original golden comparison to 2026-04-01: **unmeasured**, no run/comparison log.
- (b) Golden comparison with a generated gen27 action script: **unmeasured**, no run/comparison log.
- (c) Every regress shadows-profile key with effectsres on: **unmeasured**, no run/shadow log.
- Final `qbuild all`: **PASS**, cargo 7879 passed/0 failed/0 ignored; clippy -D warnings PASS; native build 311/311 edges; CTest 119/119.
- Gate logs: `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`, `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`.
- Formatting PASS for the three owned Rust files. Earlier focused checks: q.py launcher error (`build-runs/q/20261026-051423-26636.log`); direct cargo compile terminated before tests, excluded from verdicts.
- Effects-manager registration and month logging source unchanged; no C ABI exports added or removed.
- **effectsres is not ready for the replace list.** Do not flip cSC4EffectsResource credit. Wip queue entry only; no ready enqueue.
- Next: answer EF25, implement the resource and native collection publication, then complete (a)-(c) with parallel queue requests and the final gate.
