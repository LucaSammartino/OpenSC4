| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span1555-bridge4 (native bridge contract for cSpanRender1555; SR1555-5, SR1555-2; final round 3/3) | workflow / `re-span1555-bridge4` (spec lane) | **ready** (spec only): `docs/specs/span-render-1555.md` new §1.1 native call boundary [exact] and §1.2 shipped producers [static]; SR1555-5 answered MATCHES (§7.3 nibble order, §4.1 x87 24-bit rounding with worked examples); SR1555-6 filed (runtime producer probe); SR1555-2 narrowed, still open. No engine/tools/ABI edits | 1. Runtime hit counter on `0x0083078A` and the five method entries (SR1555-6, then SR1555-2). 2. Core binds `0x00AC182C` slots 1-5 per §1.1; pixel test only with the constructed buffer of §1.2 |

## Report

**What changed** (all in `docs/specs/`, commit da47c75e plus this file):

- `span-render-1555.md` §1.1: Init switch table `0x008308B0` (types 4 and 6 share case `0x0083077C`, vtable
  `0x00AC182C`, stored at ctx `+0xB0` by `0x008307EE`; failed alloc stores 0 and Init still returns 1, no later null
  test); vtable entries; slot 0 lifetime; thiscall table with RET values read from bytes (`0x008310F3` RET 0x10,
  `0x008311F4` RET 0x10, `0x008312C0` RET 0x14, `0x0083146D` RET 0x10, `0x008314FF` RET 0x18); ECX unused,
  EBX/EBP/ESI/EDI preserved; RenderBlt kernel cdecl 22 dwords `ADD ESP,0x58`; the seven call sites in
  `DrawPreclippedSpanList`; COM boundary (draw system slot 4, factory `0x0082B910`, Init is context slot 3 with no
  direct caller); two conformance examples.
- §1.2: static sweep (`build-runs/scan.py`): 34 colour-pair constructor calls carry no type 4/6; 37
  `PUSH 4/6` + `CALL [reg+0x14]` sites are all non-buffer code. No shipped type-4/6 RenderToBuffer producer found;
  not a proof of absence. A constructed `cGZBuffer` (CLSID `0xC470D325`, pair (4|6, 16)) is the only known pixel
  route and must be labelled "constructed buffer", never "in-game".
- §4.1: SR1555-5b: `0x0082FDB0` has no FLDCW; caller precision (24-bit main thread); five roundings per channel
  (p*K, dx*x0, dy*y0, sum, difference), then `_ftol` truncation. 4 worked examples, two differ from 53-bit
  (54886776 vs 54886778; -924958720 vs -924958694), `build-runs/gradprec.py`.
- §7.3: SR1555-5a: A4444 nibbles B 0-3, G 4-7, R 8-11, A 12-15 (masks at `0x00831566`-`0x00831594`);
  `0xF8C3` -> `0xC726`.
- §8 and questions: SR1555-5 moved to answered (MATCHES); SR1555-6 filed; SR1555-2 narrowed (594 `CALL [reg+0x6C]`
  sites, no static receiver type). `tools/questions.py index` run.

**Not done:** no runtime probe (no game run in this lane); SR1555-2 and SR1555-6 need a hit counter. REA was not
needed (re_query.py and instruction bytes only). No docs/abi rows proposed.
