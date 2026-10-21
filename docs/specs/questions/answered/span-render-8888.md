# span-render-8888: answered questions (archive)
<!-- Archive for [span-render-8888.md](../../span-render-8888.md). Lanes do not read this file. Open questions are in ../span-render-8888.md. -->

- **SR8888-2 - section 7, U2:** the keyed flag is a byte and the row is `format*2 + keyed`. Is a keyed byte other
  than 0/1 ever passed (which would select a neighbouring format's row, e.g. format 0 keyed 2 = row 2 = unkeyed
  A1R5G5B5), and is `format*2` computed in 32 bits so that `format >= 0x80000000` wraps to a low row? [both rejected
  as BadKernelIndex, together with format > 6 and mode > 11]
  **ANSWERED 2026-10-21 (re-span8888-bridge4):** MATCHES - the shipped caller never passes these: format 0..6 from `0x00830654`, kind 0..11, a `cGZBuffer` texture gives keyed 0 or 1 (`0x00826620`), format 6 forces 0; the row index is 32-bit (`0x008551DC`) and would wrap, but rejecting as BadKernelIndex is compatible (span-render-8888.md section 11 U2).
