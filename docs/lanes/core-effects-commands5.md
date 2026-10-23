| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-commands5: effects script command owners | Codex / core-effects-commands5 | in progress | Audit existing parser against effects spec command requirements. |

## Handoff
- Goal: close effects description command owners in the existing core parser; no ABI/plugin edits.
- Done: audited existing modules; documented all 22 published descriptors, shared parser sections and effects §17 scope in effects_parser/README.md.
- Done: filed EF15 (effects.md has no command inventory/algorithms; effects-parser §6 excludes all root and other nested execution); added two sequenceEffect boundary regressions.
- Focused: cargo test -p fc4-core effects_parser::tests passed 33/33; absolute-path qbuild fmt passed.
- Evidence: initial relative fmt path failed; q.py focused-test launch failed on Bash CreateFileMapping error; direct cargo succeeded.
- Next: run qbuild all once on stable source, collect terminal result, finalize report as wip (EF15 blocks whole-family closure).
- Gate: full check not run; game requests: none (no runtime/plugin changes).
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-effects-commands5-r1-manifest.json.
