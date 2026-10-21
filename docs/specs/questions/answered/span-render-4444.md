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
