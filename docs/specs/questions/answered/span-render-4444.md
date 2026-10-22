# span-render-4444: answered questions (archive)
<!-- Archive for [span-render-4444.md](../../span-render-4444.md). Lanes do not read this file. Open questions are in ../span-render-4444.md. -->

<!-- from: glue-span4444-4 (2026-10-21) -->
- ~~**SP3 - section 1 / section 7 (interop boundary for a shadow or replacement):** the spec gives the vtable
  `0x00AC185C` and the slot addresses but no call boundary a plugin may use. To wire the landed core
  (`engine/core/src/span_render4444/`) into the game without inventing anything, the glue needs: (a) the calling
  convention, stack cleanup (bytes popped) and return value of slots 1-5 (thiscall with `this` in ECX? the exact
  dword count of each argument list, e.g. slot 5 `(p1, p2, rectA, rectB, src, mode)` = 6 dwords?); (b) a
  reconciled `docs/abi/vtables.tsv` row for `nGZDrawSoftRender::cSpanRender4444` (none exists; compat entries must
  come from docs/specs or docs/abi), and whether the vtable is shared by every instance (so one slot forwarder
  covers all contexts) or copied per context; (c) whether any GZCOM path reaches the renderer instead (the SDK has
  `kcGZDrawContextAsBuffer` `0x0AB300B20` but no interface for it; is the context `+0xB0` pointer settable or
  readable through any interface?); (d) which shipped code paths create a colour-type-7 (A4R4G4B4)
  `cGZDrawContextRenderToBuffer` in the GOG 1.1.641 build under the default driver, and which scripted in-city
  action (harness `script =`) makes them call slots 1-5, so a shadow can see real native inputs.
  [glue-span4444-4: no plugin wiring; the core stays unit-tested only, no detour or vtable patch installed]~~
  **ANSWERED 2026-10-21 (re-spanbridge4444-4):** MATCHES - span-render-4444.md section 1.1: slots 1-5 are MSVC thiscall, `this` in ECX ignored, args right to left, callee pops 16/16/20/16/24 bytes (4/4/5/4/6 dwords), no return value, EBX/EBP/ESI/EDI preserved; slot 0 thiscall(flags) `RET 4`. One static vtable `0x00AC185C` shared by all 4-byte per-context objects (GlobalHeapAlloc), so patching its entries 1-5 covers every instance; proposed vtables.tsv rows in the lane's build-runs/abi-rows-re-spanbridge4444-4.tsv. Only native caller: DrawPreclippedSpanList `0x008302E0` (selection by context bytes +0x74/+0x41). No interface or GZCOM class (incl. `0xAB300B20`) exposes or sets `+0xB0`; the context comes only from factory `0x0082B910` via cIGZDrawSystem slot 4. Part (d) (shipped type-7 producer and script action) not found statically: filed as SP4.
<!-- from: glue-span4444-impl4 (2026-10-21) -->
- ~~**SP5 - section 1.2 "What a game-pixel test can use":** the pixel test through a COM-created `cGZBuffer`
  (CLSID `0xC470D325`, IID `0x20732180`) needs contracts no spec or docs/abi row publishes. Please give, for
  GOG 1.1.641: (a) the `cIGZBuffer` slot of `Init` (`0x008269B0`), its argument list (width, height, the colour-type
  pair: by value or by pointer?), callee pop count and return; (b) how a caller gets the pixel base and row stride of
  an Init-ed buffer (slot, lock/unlock pairing, arguments, pops); (c) how a plug-in obtains `cIGZDrawSystem`
  (`cRZSysServPtr<cIGZDrawSystem, 0x4B2EA3B3, 0xEB2EA3CB>` in widget-toolkit.md: is `0x4B2EA3B3` the service ID
  and `0xEB2EA3CB` the IID?) and the reference rule of slot 4's `out`; (d) the context's slot 3 `Init(buffer)`
  pop count and return, and the public context slots that set `+0x74`, `+0x41`, `+0xC4`, `+0xB4`, `+0x44` before
  slot 22 `DrawPreclippedSpanList`. [glue-span4444-impl4 creates the buffer and releases it unused; its
  original-versus-ours pixel comparison calls the five original slot functions directly on a constructed,
  plug-in-owned fixture with the published section 1.1 contract]~~
  **ANSWERED 2026-10-21 (re-span-context4):** MATCHES - span-render-4444.md section 1.3: cIGZBuffer Init is slot 3 `(width, height, type, bits)` by value, pops 0x10, returns a byte; pixels via Lock slot 6 `(flags)` / slot 34 base / slot 35 stride / Unlock slot 7 `(flags)`, pops 4/0/0/4; draw system = GetSystemService(service `0xEB2EA3CB`, IID `0x4B2EA3B3`), slot 4 `(buffer, &out)` pops 8 stores and AddRefs without releasing the old `*out` (caller owns one reference); context Init slot 3 pops 4 returns a byte; `+0x74`/`+0x44` slot 12, `+0x41` slot 8, `+0xC4`/`+0xB4` slot 7.

<!-- from: core-span4444-4 (2026-10-21) -->
- ~~**SP1 - section 6 / section 4:** RenderAlphaGradient with an alpha ramp outside 0..255 gives `a = acc >> 27` in
  17..31 (or a wrapped value). The packed R|B register form `((sRB - dRB)*a + 16*dRB + 0x808) >> 4` can then borrow
  or carry between fields. What is the exact bit result for a > 16 (masking after the shift, field placement)?
  [core evaluates each channel separately in wrapping 32-bit maths and keeps its low nibble; alpha 0xF]~~
  **ANSWERED 2026-10-22 (re-span-context-gaps4):** DIFFERS — [section 6.1](../../span-render-4444.md#61-alpha-domain-and-packed-arithmetic-sp1-exact-instruction-bytes): no alpha range check in setter or renderer; all raw alpha dwords wrap, yielding weights 0..31. The packed R/B subtraction, multiply and bias use 32-bit registers, then logical shift by 4; the word is `F000 | ((rb>>4)&0F0F) | ((g>>4)&00F0)`. Blue carries/borrows can change red at weights 17..31. Bytes `0x0084801E`–`0x0084806C`; one discriminating vector per weight plus alpha-wrap and carry/borrow vectors are in the spec. Source `0000`, destination `880F`, raw alpha `01000000` yields `FF0F`, whereas separate channels yield `F00F`. Static proof; no new native run.

<!-- from: glue-span-context-final4 (2026-10-22) -->
- ~~**SP6 - section 1.3 (fixture inputs and counting):** (a) slot 13 SetBltSource `(src, x, y, sx, sy, rect*)` and
  slot 15 SetBltTiling `(rect*)`: what do x, y, sx, sy mean (source origin and scale, or destination-to-source steps),
  and is the 16-byte rect `(x, y, w, h)` or `(left, top, right, bottom)`? (b) "count by the return address" with the
  call sites of span-render-565.md section 1.1: is each listed address the CALL instruction (return address = site +
  instruction length) or the return address itself? [the fixture uses rects at the origin, which read the same both
  ways, x = y = 0 and sx = sy in {1, 2, 0.5/0.75}; the observed site matched the expected path in all 378 cases;
  return addresses 1..7 bytes after a listed site are counted for it]. In-game evidence for SP1: through the native
  context, 4 of 6 type-7 alpha-gradient cases with alpha outside 0..255 differ (1-2 bytes, R nibble), every in-range
  case is exact (run gsctx4-r2-union2032).~~
  **ANSWERED 2026-10-22 (re-span-context-gaps4):** DIFFERS — sections [1.3.1](../../span-render-4444.md#131-source-placement-scales-and-rectangle-directions-sp6a-exact-static) and [1.3.2](../../span-render-4444.md#132-call-sites-and-exact-return-addresses-sp6b-exact-byte-backed). `x,y` are destination placement; `sx,sy` magnify the source, while stored sampling steps are their 16.16 reciprocals. SetBltSource writes a destination edge rectangle; SetBltTiling reads a source texel edge rectangle, both `(left,top,right,bottom)`. All seven listed sites are three-byte CALL instructions: use only site+3 as the renderer return address, rather than the reported site+1..7 window. Target bytes at `0x0082FFD0`/`0x0082FE70` and consumer `0x008302E0` prove the direction and field meanings; exact CALL/return/target table and nonorigin conformance cases are in the spec. The shared 1555 texture contract applies through the same context functions; draw-context.md is a separate 3D object. No new shipped hits or new game verdicts are claimed.
