| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span8888-bridge4 (native bridge contract for cSpanRender8888: vtable/callconv/lifetime, Init factory, DrawPreclippedSpanList dispatch, shipped producers; round 1) | workflow / `re-span8888-bridge4` (spec lane) | **ready** (spec only): `docs/specs/span-render-8888.md` new §1.1 native call boundary [exact] and §1.2 shipped producers [static]; SR8888-2 answered MATCHES (U2); SR8888-6 filed (runtime counter) | glue/core lane: bridge the five slots per §1.1; runtime probe SR8888-6; constructed-buffer pixel test per §1.2 |

## Report

**What changed** (`docs/specs/span-render-8888.md`):
- §1.1 Native call boundary [exact]: vtable `0x00AC1874` contents and uniqueness (each method address and the
  vtable immediate occur once; store at `0x008307CC`); per-slot argument lists, callee-pop `RET` addresses, saved
  registers (RenderBlt saves only ESI/EDI); `this` never read; RenderBlt row index, by-value copies, `ADD ESP,0x58`.
  Init (`0x00830710`): Shutdown, QI `0x20732180`, lock `0x8040`, jump table `0x008308B0` (types 8 and 9 share case
  `0x008307BE`), failed-alloc behaviour, cached surface fields, buffer stays locked. Lifetime via Shutdown/Release
  and slot 0. DrawPreclippedSpanList dispatch table (slot per condition, call sites, extra args), texture
  format map `0x00830654` (9 → 4, 8 → 5), key masks `0x00830674`, row 13 unreachable, cGZBuffer key getter
  `0x00826620` returns 0/1. COM chain: `cGZDrawContext` set-target `0x0082C830` (vtable entry `0x00AC170C`) →
  draw system slot 4 `0x0082BAB0` → factory `0x0082B910` → engine Init; neither result is tested there.
  Original-vs-ours trace with a worked RenderSolid example (constructed buffer).
- §1.2 Shipped producers [static]: screen buffer type 9 goes to SimGL (factory order); lot-config editor
  `0x00604920` makes type 8/9 cGZBuffers used as images (blit sources, rows 8-11), not targets; Gimex target 9
  load; private buffers type 1 rejected; the 9 draw-system IID sites, only `0x0082C830` calls slot 4. No shipped
  type-8/9 RenderToBuffer target found statically (virtual `+0x44` callers unresolvable) → SR8888-6.
- §11 U2 rewritten with the caller's guarantees.

**Questions:** SR8888-2 answered MATCHES (moved to `questions/answered/span-render-8888.md`); SR8888-6 filed
(runtime: does `0x008307CC` / the five method entries ever execute in a normal session). SR8888-1, -3, -4, -5 left
open (kernel behaviour, outside this bridge task). `tools/questions.py index` run.

**Method/limits:** static only. REA had no app open (`target_unavailable`), so all evidence is from
`re_query.py asm`, decomp/ and raw image reads (`build-runs/rd8888.py`, gitignored). No code, no engine edits,
no ABI rows proposed (the native ABI is the shipped vtable; no docs/abi change needed).

**Commits:** ac436894 lane file, e7874e5f §1.1, fe208b62 §1.2 + questions, then this report.

**Next steps:** a glue/core lane bridges slots 1-5 to the existing kernels per §1.1 (callee-pop sizes, ECX
ignored, ESI/EDI/EBX/EBP preserved); a runtime lane answers SR8888-6 with counters at `0x008307CC` and the method
entries; pixel tests use the constructed-buffer recipe of §1.2 and say so.
