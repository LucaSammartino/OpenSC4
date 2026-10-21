# span-render-4444: open questions
<!-- Questions about [span-render-4444.md](../span-render-4444.md). Append new entries at the END; when one is answered, move it to answered/span-render-4444.md. Rules: README.md. -->

<!-- from: core-span4444-4 (2026-10-21) -->
- **SP1 - section 6 / section 4:** RenderAlphaGradient with an alpha ramp outside 0..255 gives `a = acc >> 27` in
  17..31 (or a wrapped value). The packed R|B register form `((sRB - dRB)*a + 16*dRB + 0x808) >> 4` can then borrow
  or carry between fields. What is the exact bit result for a > 16 (masking after the shift, field placement)?
  [core evaluates each channel separately in wrapping 32-bit maths and keeps its low nibble; alpha 0xF]
- **SP2 - section 2:** a span record whose `end - start` exceeds the pixels left in the row (or whose `end` is below
  `start`, wrapping to a huge width) writes linearly past the row end in the original. Do callers ever pass such
  records, or may a reimplementation require `x + width <= 65536` on one row? [core walks linearly from (x, y) and
  rejects any byte outside the supplied buffer before writing anything]
<!-- from: glue-span4444-4 (2026-10-21) -->
- **SP3 - section 1 / section 7 (interop boundary for a shadow or replacement):** the spec gives the vtable
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
  [glue-span4444-4: no plugin wiring; the core stays unit-tested only, no detour or vtable patch installed]
