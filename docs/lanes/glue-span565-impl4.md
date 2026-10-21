| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span565-impl4: plugin glue enable=span565 (cSpanRender565 vtable 0x00AC1844 bridge) | `glue-span565-impl4` / (engine lane) | **ready**, ABI additive (5 fc4_span565_* exports), default off; gate cargo 3987/0, clippy 0, ctest 65/65; game span565-t2 clean ref 0, trace EXACT 20/20 (constructed buffer), shipped calls 0; SR565-7, SR565-8 filed | 1. spec lane: SR565-7 (fixture conventions, setter slots), SR565-8 (texture record) so RenderBlt can be traced through DrawPreclippedSpanList. 2. SR565-5/6 still open. 3. coordinator: regenerate QUESTIONS.md. |

## Report

**What changed.**
- core `span_render565`: `dest_extent`, `blt_source_extent` (pure; exact address hull, upper bound for keyed slots) + test (7d646510).
- capi `span565.rs`: `fc4_span565_render_{solid,gradient,alpha,alpha_gradient,blt}` on the native records, typed
  window = hull of the call, decline codes 1-7 (nothing written) (04b3cea8); fc4.h records + layout asserts.
- plugin `FC4Span565Bridge.{h,cpp}` (5d3459ee, 486eb873): `enable = span565` (default off) checks all 6 entries of
  0x00AC1844 against the published originals, writes slots 1-5 with __fastcall/thiscall forwarders at PostAppInit,
  restores at PreAppShutdown; declined calls go to the native method. Counters per slot split shipped/constructed
  (trace thread). `span565_trace = 1`: COM-created cGZBuffer Init(5,3,5,16) stride 12, context from draw system
  slot 4, native Init -> 1 with renderer vtable 0x00AC1844, DrawPreclippedSpanList native-forced vs ours for solid,
  alpha, gradient, alpha-gradient, every byte incl. padding; RenderBlt by direct slot-5 calls (texture cGZBuffer
  type 9/32, kinds 0,1,2,3,4,6,8,11 x key 0/1); Shutdown then releases.

**Game.** span565-t1: clean, EXACT, ref_warnings=6 (created objects not acquired in the audit; fixed). span565-t2
(turbo to 2032): clean, ref_warnings=0, trace EXACT 20/20, constructed ours 1/1/1/1/16, shipped 0/0/0/0/0
(no shipped 565 call seen, consistent with §1.2; not a proof, SR565-5).

**Literal readings / questions.** SR565-7: draw-system slot 4 taken as thiscall pop 8 bool, Shutdown no args,
context fields written directly (no published setters). SR565-8: texture record source unknown, so slot 5 is not
traced through DrawPreclippedSpanList. SR565-6 (lock leak) untouched: the bridge does not change Init/Release.

**Gate.** Baseline cargo 3980/0, ctest PASS; final cargo 3987/0, clippy 0, ctest 65/65.
