| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span565-4 | workflow / `re-span565-4` | in progress | write docs/specs/span-render-565.md |

## Handoff
- Goal: spec nGZDrawSoftRender::cSpanRender565 (5 methods 0083DE50..0083E210 + its blit table) in docs/specs/span-render-565.md, verified by unicorn emulation (harness in docs/specs/proofs/).
- Done: lane file 61e826f2; proofs harness 2b48399d + alias (this commit): span565_methods 5/5 PASS 500; span565_blt 168/168 PASS 300 (also `via` RenderBlt 100); span565_sweep 168/168 (4 colours, all 16-bit texels); span565_alias 168/168 (200).
- Next: write docs/specs/span-render-565.md (sections: common, Solid, Alpha, Gradient, AlphaGradient, RenderBlt+6.1 caller, table, model 7.x, proofs, open points); file SR565-1 (gradient reachability).
- Facts: table rows 0x00AC1E28 (row i = 0x00AC1DF8-0x30i); 141 distinct bodies (139 in range + 0x00848160, 0x00833730); helper 0x00845450 = blend; REA unavailable (no app open), used decomp/ + asm.
- Facts (old): sibling lanes re-span1555-4 / re-span4444-4 have a unicorn harness on their branches (build-runs/emu1555*.py); reuse approach only, verify every 565 formula from bytes.
