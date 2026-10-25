| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-picview7: the zoomable picture view and the album picture, its image resampler and kernels, the texture tile packer, picture-view.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-fwmisc7b) | **ready** (docs only). Six owners claimed: specified +10,682 bytes (+41 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 41 functions were read in the decompile. Read in the disassembly: the view interface table `0x00ABA700` and its
zoom, scroll, centre and window-to-image bodies (shared owners, described for completeness), `CalculateAllMetrics`,
`GZPaint`, the key handler, the file loader and writer, the fit-to-window path, the resampler wrapper and worker, the
Catmull-Rom and tent kernels and their double constants, `GenerateKernelArray`, `GenerateKernel`,
`ConvertKernelToInteger`, the GVExtra size switch and double click, and `cRZTileSetup::Init` with its caller
`cGZBufferSimGLCache::InitCache`. `cGZGraphicSystemW32` was left alone: gz-graphics.md GZG3 is being worked upstream.
Written on 2026-10-24.

### What changed

- New docs/specs/picture-view.md.

### Quirks found

- `ConvertKernelToInteger` adds the truncation excess instead of subtracting it.
- Fit-to-window resamples into a buffer of exactly the window's size (aspect ratio lost).
- Writing a picture whose name lacked the extension deletes the file at the extension-less name.
- Centred-mode metrics shift by the unscaled half size; `CenterOn` tests the unscaled picture against the window.
- `ZoomOut` compares its factor with the minimum zoom; the view consumes every key.
- The album picture's `Init` calls `SetSizeInOut(1)` after the constructor already set large mode (no effect).

### Coverage

- tools/coverage-map.tsv: `cWinBMPView`, `cRZTileSetup` rows filled; `fn:Resample`, `fn:GenerateKernelArray`,
  `fn:ConvertKernelToInteger` added after `cWinBMPView`; `cWinBMPViewGVExtra` → picture-view.md, snapshots.md.
- Specified bytes 4,911,946 → 4,922,628 (+10,682, 88.2%, `cov_local.py`).
