| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gimex7: the Gimex image factory, reader and writer, gimex.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-descrows7) | **ready** (docs only). Codec selection, the reader/writer wrappers, the 19 factory entries, loading and translation into each colour type (incl. the 4444 error-diffusion dither and palette paths), saving with the 32-bit conversion; owners claimed: specified +7,756 bytes (+71 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 71 functions were read in the decompile. Read in the disassembly: the four vtables (`0x00ACE530`, `0x00ACE4FC`,
`0x00ACE57C`, `0x00ACE5B0`), the 16/32-bit converters, the save converter `0x0090BA37`, the dither `0x0090BF3C` and
the save path `0x0090BD07`. The Gimex library (`_GIMEX_*`) is left as a library contract. Written on 2026-10-24.

### What changed

- New docs/specs/gimex.md.

### Quirks found

- 32-bit images loaded into R5G6B5 buffers get half-intensity green (5 bits placed one bit low).
- Saving an A4R4G4B4 buffer advances the source one byte per pixel and mixes nibbles.
- Palettised images into 32-bit buffers: type 9 gets alpha 0, type 8 the colour shifted by 8 bits.
- The dither's error lanes can carry into each other, and the start entry of each row keeps old error.

### Coverage

- tools/coverage-map.tsv: `cGZGimexFactory`, `cGZGimexReader`, `cGZGimexWriter` → gimex.md.
- Specified bytes 4,888,939 → 4,896,695 (+7,756, 87.7%, `cov_local.py`).

### Not covered

- The Gimex library itself and the record-as-file object (`cGZDBSerialRecordAsIGZFile`, class `0x0A32C0DA`).
