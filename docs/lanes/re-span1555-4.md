| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span1555-4 | workflow / `re-span1555-4` | **ready** (specs only, no ABI rows): `docs/specs/span-render-1555.md` complete — all 172 owner functions [exact] or [verified]; 168/168 blit slots emulation-verified; SR1555-1/3/4 answered | SR1555-2 (is a 1555 gradient ever drawn) needs an indirect-call trace or in-game capture |

## Report

**What changed (round 2, from d80e0421).** Commits `1fc93d06` (gradient producer, blit source bias, stretch
caller values, tiled addressing), `48e8ed68` (all slots verified, keyed rows, alpha/modulate per source,
questions), `62af8ea6` (script path note), plus this report.

- §4.1: `SetGradientTransform` `0x0082FDB0` fills the 0x30 record: 12 floats × 16711680 (255·65536, double
  `0x00AC17C8`), float-rounded then `fistp`; start + 0x8000 shifted to pixel (0,0), `_ftol`; null → `+0x74 = 0`.
- §5: exact packed blend for `w > 32` (`0x008313D9`–`0x00831421`).
- §6.1: `src.+0x14` is bytes per texel (not a palette); plain mode biases the source by `(ox, oy)`;
  `SetBltSource` `0x0082FFD0` stores du/dv = R(65536/s), ox/oy integer (scale 1) or pre-scaled 16.16 (stretch).
- §7.3: keyed rule for every row (raw texel & caller mask == key → skip); row 13 uses an empty mask.
- §7.4: 8888 expansion table per source; ConstantAlpha never uses source alpha (col 1 calls `0x0083C490`,
  its only 14 call sites); ModulateAlpha weights by the modulated alpha.
- §7.5/§7.6: stretch caller `b = {(du>>1)−ox, (dv>>1)−oy, du, dv}`; tiled caller (pointer to tile origin,
  size = tile, `u0 = (M − M mod TW − ox) mod TW`, M = 0x7FFFFFFF) and body (mod-wrap per span, reset per pixel).
- §7.7: one rule for all 168 slots and the verification.

**Measured.** Unicorn 2.1.4 (installed with `pip --target build-runs/pylib`, not committed) runs the shipped
bytes: `build-runs/emu1555.py 3000` → 168/168 slots match, 504 168 cases; `build-runs/emu1555m.py 5000` →
Solid, Alpha, Gradient, AlphaGradient, RenderBlt dispatch all match. Owner range: 172 = 166 slot bodies +
5 methods + `0x0083C490`; none unmatched. Not covered: `n = 0`, stretch out of range / 32-bit wrap, tiled
`u0 ≥ TW`. `build-runs/norm.py` (capstone normalised listings) shows no two slots instruction-identical
(register allocation differs), so the proof is behavioural, not CFG-identity.

**Questions.** Answered SR1555-1, -3, -4 (MATCHES). Open: SR1555-2 (no direct caller of `0x0082FDB0` or the
base forwarder `0x0082C360`; reachable only through `cGZDrawContext` vtable `+0x6C`).

**Tools.** REA not used (export, decomp and emulation sufficed). Scripts committed with `git add -f` in
build-runs/ (emu1555.py, emu1555m.py, norm.py) so the check is reproducible.
