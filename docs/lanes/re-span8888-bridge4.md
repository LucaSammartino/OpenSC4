| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span8888-bridge4 (native bridge contract for cSpanRender8888: vtable/callconv/lifetime, Init factory, DrawPreclippedSpanList dispatch, shipped producers) | workflow / `re-span8888-bridge4` (spec lane) | in progress | see Handoff |

## Handoff
- Goal: span-render-8888.md new §1.1 native call boundary [exact] and §1.2 shipped producers of types 8/9 into RenderToBuffer; questions for gaps.
- Template: span-render-1555.md §1.1/§1.2 and span-render-4444.md §1.1/§1.2 (shared Init/factory code; re-read 8888 bytes).
- Done: spec §1.1 native boundary [exact] (commit 2). Chain found: cGZDrawContext slot17 0x0082C830 -> drawsys slot4 -> factory -> Init.
- Next: §1.2 shipped producers of type 8/9 targets reaching a cGZDrawContext set-target (candidates: lot-config 0x00604920 types 8/9, Gimex target 9 at 0x007BFDB4, pair-ctor 0x0044A490 type-9 calls); then answer SR8888-3 (row 13 unreachable) / SR8888-2 (flag 0/1 from cGZBuffer 0x00826620).
- Tools: build-runs/rd.py (dw/find on the image). REA: no app open -> static fallback only.
