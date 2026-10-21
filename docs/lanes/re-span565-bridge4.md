| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span565-bridge4 | workflow / `re-span565-bridge4` | in progress | add span-render-565.md §1.1/§1.2 native bridge |

## Handoff
- Goal: span-render-565.md native call boundary (vtable 0x00AC1844, 5 slots, lifetime, Init case 0x00830792,
  context fields, DrawPreclippedSpanList dispatch) + shipped-producer route, re-derived from 565 bytes.
- Done: nothing yet.
- Next: re_query vtable/asm for 0x00AC1844, 0x00830792, method epilogues.
