| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span565-bridge4 | workflow / `re-span565-bridge4` | in progress | add span-render-565.md §1.1/§1.2 native bridge |

## Handoff
- Goal: span-render-565.md native call boundary (vtable 0x00AC1844, 5 slots, lifetime, Init case 0x00830792,
  context fields, DrawPreclippedSpanList dispatch) + shipped-producer route, re-derived from 565 bytes.
- Done: spec §1.1 (vtable/slots/RET/lifetime/Init/context fields/DPSL dispatch) and §1.2 (producer sweep, fixture
  route), questions SR565-5/6; commit below.
- Next: verify conformance examples with docs/specs/proofs harness if cheap; finish report + merge-queue.
- Facts: build-runs/raw.py (dwords/occurrences), scan5.py (type-5 sweep: only 0x00826472).
