| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span-context4 (RenderToBuffer draw-context ABI: SP5, SR1555-7) | workflow / `re-span-context4` (spec lane) | in progress | see Handoff |

## Handoff

- Goal: publish exact cGZDrawContextRenderToBuffer (vtable 0x00AC17D0) creation/Init/state/setters/Lock/Unlock and
  DrawPreclippedSpanList ABI for buffer types 4/6/7; answer SP5 (span-render-4444) and SR1555-7 (span-render-1555);
  specify a native-context fixture route (constructed vs shipped call counts).
- Done: none yet.
- Next: dump vtable 0x00AC17D0 (re_query vtable), read setters, cGZBuffer Init 0x008269B0, draw system slot 4.
- Facts: span-render-565.md §1.1 already gives Init/Shutdown/Release order (Shutdown unlocks 0x8040; Release does not).
