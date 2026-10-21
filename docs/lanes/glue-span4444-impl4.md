| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span4444-impl4: native bridge for cSpanRender4444 (`enable = span4444`) | workflow / `glue-span4444-impl4` | in progress | see Handoff |

## Handoff

Goal: bridge vtable `0x00AC185C` slots 1-5 (span-render-4444.md §1.1) to the core, `enable = span4444`
(default off); original-versus-ours pixel comparison on a constructed fixture; file questions for the unpublished
cGZBuffer / draw-context contracts.

Done:
- CAPI `engine/capi/src/span_render4444.rs` (fc4_span4444_solid/alpha/gradient/blt), uncommitted at first write.

Next: lib.rs `pub mod span_render4444;` + fc4.h decls; compat SpanRender4444Entries; plugin
FC4SpanRender4444.cpp (forwarders __fastcall = thiscall callee-pop 16/16/20/16/24, fixture compare at PostAppInit);
subsystem "span4444" (0 classes, default off); SP5 question; gate; game run.

Facts: baseline `qbuild all` started at the first commit (result pending).
