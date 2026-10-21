| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span8-bridge4 | workflow / `re-span8-bridge4` | **ready** (specs + proof, no ABI rows): `docs/specs/span-render-8.md` §10 bridge [exact] — renderer object/vtable/callee-pop/lifetime, context `Init` factory selection, type-2 `cGZBuffer` format/Init/Lock/colour map, `DrawPreclippedSpanList` dispatch, COM route; `span8_bridge.py` PASS; SR8-1 answered (DIFFERS: solid byte always 0), SR8-2 narrowed, SR8-4 filed | SR8-2 (data-typed buffers reaching CreateDrawContext as type 2), SR8-4 (antialiased path on type 2), SR8-3 (row 13 texel reads) |

## Report

**What changed** (all in `docs/specs/`):
- `span-render-8.md` §10 (new): 10.1 the 4-byte stateless renderer object and the six vtable slots with argument
  lists, callee-pop bytes (4/0x10/0x10/0x14/0x10/0x18) and returns (none read); 10.2 lifetime (created by
  `Init` 0x00830710, deleted by `Shutdown` 0x0082FF20 and final `Release` 0x008306C0; constructor 0x00830690);
  10.3 `Init` step order, jump table 0x008308B0 (type 2 → 0x00AC188C; 3 and out-of-range fail, leaving the target
  locked; bpp half ignored), destination record copy; 10.4 type-2 `cGZBuffer` (vtable 0x00AC1400): Init stride
  `(w*bpp/8+3)&~3`, Lock/Unlock counters and flags, colour map 0x00825DA0 returning 0 for type 2 (also 0x0082AD30);
  type 3 is the paletted 8-bit, type 2 unpaletted; 10.5 COM route: cGZDrawSystem::Init 0x0082BC90 registers the
  software factory (vtable 0x00AC162C, IID 0x6B2EA76C, create 0x0082B910, 0xD4 bytes, uninitialised context),
  CreateDrawContext 0x0082BAB0 tries factories in order; Draw 0x00831000 only does antialiased primitives through
  FillAntialiasedPolygon (bypasses the span renderer); cGZDrawContext rasterisers call DrawPreclippedSpanList
  (`+0x58`, `ret 8`) whose slot choice is tabled; worked fixture route; 10.6 shipped producers.
- §8 proofs table: `span8_bridge.py` row. §9 open points updated. Intro points to §10.
- `proofs/span8_bridge.py`: runs shipped buffer Init, colour maps, context ctor/Init for types 0–11, SetColorTable
  and DrawPreclippedSpanList (solid, alpha flag) on constructed type-2 fixtures; only the global heap alloc/free
  are stubbed. Result 2026-10-21: PASS (needs unicorn in build-runs/pylib).

**Questions**: SR8-1 answered (moved to questions/answered/span-render-8.md, DIFFERS: solid/alpha fills on a
type-2 target store 0). SR8-2 kept open, narrowed: the only literal `Init(...,2,8)` is
cSC4WinRegionScreen::MakeWholeRegion 0x007AEDD0 (heightfield, set-pixel writes, no context); data-typed buffers
(resource loaders, screen buffer) not traced. SR8-4 filed (antialiased path on type 2). SR8-3 untouched.

**Fixture vs shipped**: every route in §10 and §8 is a constructed fixture; no shipped draw into a type-2 target
is known. REA was not used (re_query.py, decomp/ and the exe bytes sufficed).

**Next**: SR8-2: trace callers of cIGZDrawSystem slot 4 (`+0x10`) and the buffer-resource colour-type sources;
SR8-4 belongs to a context spec (FillAntialiasedPolygon 0x008308D0).
