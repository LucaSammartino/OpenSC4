| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-spanbridge4444-4 | workflow / `re-spanbridge4444-4` | **ready** (spec only): SP3 answered in `docs/specs/span-render-4444.md` §1.1 (native call boundary of `cSpanRender4444`); SP4 filed (type-7 producer) | SP4 needs a runtime breakpoint on `0x008307A8`; reconcile the proposed vtable rows |

## Report (2026-10-21)

**What changed**
- `docs/specs/span-render-4444.md` new §1.1 "Native call boundary": 4-byte per-context object (GlobalHeapAlloc,
  vptr only) pointing at one shared static vtable `0x00AC185C` (immediate occurs once; slot entries 1-5 each occur
  once in the image), so patching entries 1-5 covers every A4R4G4B4 instance. Slots 1-5 are MSVC thiscall, `this`
  ignored, callee pops 16/16/20/16/24 bytes (4/4/5/4/6 dwords), no return value, EBX/EBP/ESI/EDI preserved; slot 0
  thiscall(flags) `RET 4`, frees via GlobalHeapFree `0x005E5620`. Lifetime: Shutdown `0x0082FF20` / Release
  `0x008306C0`. Only native caller: `DrawPreclippedSpanList` `0x008302E0` (slot selection table by context bytes
  `+0x74`/`+0x41`, argument sources `+0x78`, `+0x44`, `+0xB4`, `+0xC4`; blit sites `0x008304D1`, `0x0083053B`,
  `0x00830591`). No interface exposes `+0xB0`; the context is created only by factory `0x0082B910` (vtable
  `0x00AC162C`, registered by `cGZDrawSystem::Init`, reached via `cIGZDrawSystem` `0x00AC1668` slot 4 `0x0082BAB0`);
  `0xAB300B20` is not on that path. §7 and §11 cross-reference it.
- Questions: SP3 archived to `docs/specs/questions/answered/span-render-4444.md` (MATCHES); SP4 filed (which shipped
  path builds a type-7 RenderToBuffer target; static sweeps found no literal 7 at the 34 colour-pair constructor
  calls or at draw-system CreateBuffer calls). `docs/specs/QUESTIONS.md` regenerated.
- Proposed ABI rows (not reconciled into docs/abi): `build-runs/abi-rows-re-spanbridge4444-4.tsv` (6 rows,
  `docs/abi/vtables.tsv` format, vtable `0x00AC185C`).

**Method**: all from bytes (`re_query.py asm --bytes`) and a raw image scanner `build-runs/scan.py` (immediate,
call-target, disp32 and push-imm sweeps). REA was not needed.

**Next**: SP4 by a runtime breakpoint on `0x008307A8` (Init, type-7 branch) in a scripted UI session; then a shadow
could forward the five slots by patching `0x00AC1860`-`0x00AC1870`.
