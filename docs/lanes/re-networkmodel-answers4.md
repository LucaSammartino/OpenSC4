| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-networkmodel-answers4 (NO21-NO23, network-occupants.md §10 model instance) | `re-networkmodel-answers4` / (RE spec lane) | in progress | answer NO21-NO23 from native bytes |

## Handoff

- Goal: answer NO21 (Deform), NO22 (FlipAndLight/Clip/Prelight), NO23 (model-instance lifetime) in
  docs/specs/network-occupants.md §10, move entries to questions/answered/, run questions.py index.
- REA: "No app is open" (2026-10-22) -> static fallback (re_query.py asm + decomp/).
- Facts so far: the four bodies have NO vtable refs; they are non-virtual members called only from
  cSC4NetworkOccupantWithPreBuiltModel::GetModelInstances 0x0061E8A0 (Clip 0x0061EE60, Prelight
  0x0061EE73/0x0061EEE9/0x0061F359, Deform 0x0061EF26, FlipAndLight 0x0061EF35/0x0061F368).
  Instance size 0xE4 (new at 0x0061e9xx, ctor 0x00614A30 called 0x0061E9B7 and 0x0061F06C).
  0x00614350 "Prelight" = processor(+0x9C).SetModel(instance's +0x20 virtual-base view).
  Plain GetModelInstances 0x0061AF90: emits the piece's own +0x38 cS3DModelInstanceBasic, AddRef'd.
  Two cached instances: prebuilt +0x1C4 (main model) and +0x1C8 (from +0x19C, second model).
- DONE (commit 2): §10.1-10.3 written; NO21/NO22/NO23 answered DIFFERS, moved to answered/; new NO25 (Clip
  kept side/interp) and NO26 (cache release in dtor/Shutdown) filed; questions.py index run.
- Next: optional NO26 (prebuilt dtor/Shutdown release of +0x1C4/+0x1C8), then finish report + merge-queue.
- Helper: build-runs/dd.py VA count [f] dumps exe dwords/floats.
