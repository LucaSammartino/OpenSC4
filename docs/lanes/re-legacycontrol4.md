| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-legacycontrol4: spec cGZLegacyControl (new docs/specs/legacy-control.md) | Claude / .claude/worktrees/re-legacycontrol4 | in progress | see Handoff |

## Handoff
- Goal: new docs/specs/legacy-control.md for owner cGZLegacyControl (59 fns, ~104 KB); docs only.
- Done: spec v1 (layout, slots, lifecycle, vocab tables all 21 fns, SetParameters, readers) commit after 4274d299.
- Next: §8 done; per-type interpreters small per-type interpreters (Slider 403, Folders 465, FlatRect 475, Outline 485, BMP 492).
- Tools: build-runs/vocab2.py (asm word extractor), vocabmd.py (tables).
- Facts: decomp/cGZLegacyControl.c (9338 lines); vtables 00ad5360 (11), 00ad5324 (8); ImplementInterface ICF-folded at 00935ea0.
