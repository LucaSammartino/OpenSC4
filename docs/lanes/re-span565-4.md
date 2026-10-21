| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span565-4 | workflow / `re-span565-4` | **ready** (specs only, no ABI rows): `docs/specs/span-render-565.md` complete — all 145 owner functions [exact] or [verified]; 168/168 blit slots verified by emulation (random, via-RenderBlt, texel sweep, aliased overlap); SR565-1 filed | SR565-1 (is a 565 gradient ever drawn) needs an indirect-call trace or in-game capture |

## Report

**What changed.** Commits `2b48399d` (unicorn proof harness: loader, methods, blit slots, texel sweep), `b9ee7227`
(aliased-overlap harness), `f54a4b2c` (spec `docs/specs/span-render-565.md`, question SR565-1, QUESTIONS.md index).
Harness lives in `docs/specs/proofs/span565_*.py` (RE-only, no game behaviour; needs unicorn + bin/SimCity4.exe).

**Spec content.** §1 span/record/stride/no-clip/overlap rules; §2 RenderSolid (low 16 bits verbatim); §3
RenderAlpha weight `(a8·0x818180)>>26` and the packed blend of helper `0x00845450`; §4 gradient record, its producer
`0x0082FDB0` (re-read from bytes) and the 565 per-pixel rule (R/G exact top bits, B start SHR 3 + step SAR 3 drift,
worked example); §5 AlphaGradient (`w = (V3·0x84)>>26`, up to 63 out of range); §6 RenderBlt dispatch (table
`0x00AC1E28`) and the caller's fill (format/key tables `0x00830654`/`0x00830674`, kind, plain bias, stretch
half-pixel origin, tiled `K_W` origin) re-read from `0x008302E0`; §7 table of 168 slots (141 distinct bodies, folds
listed), per-source conversion, ConstantAlpha (`w = ca>>3`, never full), Modulate/ModulateAlpha, key rule, stretch
(logical shifts, 156 checked) and tiled addressing.

**Measured results (2026-10-21).** `span565_methods.py 500`: 5/5 PASS. `span565_blt.py 300`: 168/168.
`span565_blt.py via 100`: 168/168. `span565_sweep.py 4`: 168/168 (all 16-bit texels). `span565_alias.py 200`:
168/168 (a snapshot-read model fails 148 slots, so the test discriminates).

**Notes.** REA was unavailable (no app open in the shared session); used decomp/ region files, re_query asm and
direct emulation. 1555/4444 harness ideas were reused for structure only; every 565 formula was derived from 565
bytes and passes only against them (e.g. the 565 gradient differs from 1555's).

**Questions.** SR565-1 filed in `docs/specs/questions/span-render-565.md`. The question tool parses `SR565` as prefix
`SR` + number (next free shows SR566), the same as the sibling SR1555/SR4444 entries; the index will need a
regenerate when the three lanes merge.

**Next.** SR565-1: trace indirect calls to `cGZDrawContext` vtable slot `+0x6C` or capture an in-game gradient.
