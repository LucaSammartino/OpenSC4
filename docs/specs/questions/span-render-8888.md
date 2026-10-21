# span-render-8888: open questions
<!-- Questions about [span-render-8888.md](../span-render-8888.md). Append new entries at the END; when one is answered, move it to answered/span-render-8888.md. Rules: README.md. -->

<!-- from: core-span8888-4 (2026-10-21) -->
- **SR8888-1 - sections 7, 9.1:** when the blit source overlaps the destination (same buffer), does every kernel
  process one pixel at a time left to right, reading the texel (and destination) of pixel i only after pixel i-1 has
  been stored? Or do any kernels (for example the MMX modulate columns, or the 2-byte formats) load two or more
  texels/pixels per step, or run right to left, so that a texel overwritten earlier in the same span is read with
  its old value? [strictly one pixel at a time, left to right, spans in list order, reads against live memory;
  tests `overlap_*` pin it]
- **SR8888-3 - section 9.2, A8 keyed rows:** with key 0 the kernel draws nothing; does it still walk the spans and
  read texels (relevant only for faults on a bad source pointer), and with key != 0 is the keyed A8 kernel's output
  identical to the unkeyed row for every op? [no reads observable; this API still bounds-checks every texel address
  and treats key != 0 as the unkeyed row]
- **SR8888-4 - section 6:** section 6 says the fields "interfere (garbage colour)" for w > 256, but in the cases
  computed here by the packed formula each channel equalled the per-channel rule taken mod 256 (d = 0,
  s = 0x00FFFFFF, w = 512 gives 0xFFFEFEFE; d = 1, s = 0, w = 65535 gives 0xFF000001). Is there a concrete (d, s, w)
  where a channel differs from `floor((256*d + w*(s - d) + 128)/256) mod 256`? [the packed formula is implemented
  verbatim, so the answer does not change the code]
- **SR8888-5 - section 9.1, U4:** stretch rows are `base + v*stride` and texels `row + bpp*(u >> 16)` with 32-bit
  pointer arithmetic in the original. Should an address that wraps around 2^32 back into the buffer be honoured?
  [no: this API computes addresses exactly in 64 bits and rejects any address outside the buffer as
  SourceOutOfBounds/DestOutOfBounds before writing anything]

<!-- from: re-span8888-bridge4 (2026-10-21) -->
- **SR8888-6 - section 1.2:** in a normal session (load a city, open the lot-configuration editor, browse menus,
  take a snapshot), is the 8888 vtable store `0x008307CC` in `cGZDrawContextRenderToBuffer::Init` ever executed,
  and are the entries `0x00854E10`, `0x00854E60`, `0x00854F70`, `0x00855040`, `0x008551D0` ever hit? If so, which
  caller of `cGZDrawContext` set-target (`0x0082C830`, return address of its caller) supplied the type-8/9 target?
  [static sweep found none; until answered, treat every 8888 run as "constructed buffer"]
