| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span565-4 | workflow / `re-span565-4` | in progress | write docs/specs/span-render-565.md |

## Handoff
- Goal: spec nGZDrawSoftRender::cSpanRender565 (5 methods 0083DE50..0083E210 + its blit table) in docs/specs/span-render-565.md, verified by unicorn emulation (harness in docs/specs/proofs/).
- Done: lane file.
- Next: read RenderSolid..RenderBlt bytes, locate 565 blit table, write harness.
- Facts: sibling lanes re-span1555-4 / re-span4444-4 have a unicorn harness on their branches (build-runs/emu1555*.py); reuse approach only, verify every 565 formula from bytes.
