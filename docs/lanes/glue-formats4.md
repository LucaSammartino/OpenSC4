| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-formats4 (GZCOM glue: data formats, DBPF/exemplars) | `glue-formats4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: GZCOM glue for the largest coherent supported data-formats owner (exemplar factory type 0x6534284A per spec §8), using engine/formats parsers; or precise contract questions (DF-n) and stop blocked.
- Done: worktree reset to main d286d363; capi fc4_dbpf_* (6d935c1d, additive, no ABI bump); plugin FC4FormatsShadow (29eb480d), key `formats_shadow = 1` (no "formats" subsystem exists to enable; nothing replaced: spec §8 says plug-in mode keeps the originals, a factory replacement needs RegisterObjectFactory/record contracts the spec lacks).
- Gate (qbuild all at 29eb480d): cargo 3591/0, clippy 0, ctest 62/62.
- Next: wait gf4-formats-shadow-1 verdict (build-runs/game-results/), grep log for "formats shadow"; differences -> DF questions.
- Facts: request gf4-formats-shadow-1 outstanding (60 s, Konradshohe).
