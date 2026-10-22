| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-textures-contractfix4: texture/material contract corrections | Codex / `core-textures-contractfix4` (engine lane) | in progress | 1. Read contracts and baseline. 2. Correct owner and distinguishing tests. 3. Final gate and review queue. |

## Handoff
- Goal: correct existing textures_materials owner to published TB4–TB7 contracts; no native/default promotion.
- Done: reserved detached worktree reused; clean initial status; TB4–TB7 focused contracts read.
- Done: e9ee879f signed power, masked LOD, wrapping layout and binding references (before call 30).
- Done: factory wrapping counters, instance-zero cache/reuse, bulk failure deletion, source-frame/masked-level/null uploads.
- Done: material 16-bit acquisition/release, reference/value/stage assignment, forward range cleanup, manager false slots/base destruction.
- Done: independent distinguishing vectors added; focused quick and fmt invoked, logs pending.
- Done: 2a906396 factory/material correction and independent vectors; formatting passed five touched Rust files.
- Done: order-independent repeated-alias restore now reloads the current record key once per table entry; distinct-record order withheld under TB8.
- Done: f471bdc8 stable source; fmt PASS five files. Baseline all interrupted after 639 passed/0 failed completed results; C API second-thread clock test stalled over 60 seconds. No baseline clippy/native completion.
- Next: final all on f471bdc8 with RUST_TEST_THREADS=1; record exact counts and review queue.
- TB8 filed: native hash/growth/chain rules absent; distinct-record restore explicitly unsupported.
- Baseline wrapper session 62872 interrupted with Ctrl-C; focused quick native build still pending.
- Scope: textures_materials and independent conformance/tests only; no game runs or shared ABI edits.
- Family cap: parent core-textures-materials4 round 1 landed train84; this reservation logical 2/3, max one round here.
- TB1 interface identity and TB2 settings producer remain unknown.
