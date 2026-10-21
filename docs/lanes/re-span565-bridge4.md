| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span565-bridge4 | workflow / `re-span565-bridge4` | **ready** (spec only, no ABI rows): `docs/specs/span-render-565.md` §1.1 native call boundary [exact] + §1.2 producer sweep and fixture route; proof `span565_bridge.py` PASS; questions SR565-5, SR565-6 | SR565-5 runtime counters at `0x00826472` / `0x008307A0`; SR565-6 lock-leak compat decision; core lane may add a constructed-buffer original-vs-ours trace per §1.2 |

## Report

**What changed.** `docs/specs/span-render-565.md`:
- §1.1 (new): compiled vtable `0x00AC1844` (6 entries read from the image; each method dword occurs once, the vtable
  immediate once at `0x008307A0`), per-slot arguments, callee pops with RET addresses (`0x0083DE93`, `0x0083DFA5`,
  `0x0083E06D`, `0x0083E20C`, `0x0083E29F`; slot 0 `0x0082FF1C`), no return value, ECX never read, callee-saved
  registers; object/lifetime (Init `0x00830710` steps incl. QI `0x20732180`, lock `0x8040`, jump table `0x008308B0`
  type 5 → `0x00830792`, null-allocation case; Shutdown, final Release, slot 0); destination record and context
  fields written by Init (`+0x78..+0x90`, `+0x0C/+0x10`, `+0xA4`, `+0x75`) and the cGZBuffer (`0x00AC1400`) getters
  and lock/unlock behind them; DrawPreclippedSpanList's seven call sites with slot and argument sources; texture
  lock/unlock; two boundary conformance examples.
- §1.2 (new): static sweep (`build-runs/scan5.py`, not committed): the only type-5 producer is the cGZBuffer type
  resolver fallback `0x00826472` (types 1/3 become 5/16 only when no `cIGZGraphicSystem` service exists); no shipped
  path to a type-5 RenderToBuffer target found. Constructed-fixture route (cGZBuffer CLSID `0xC470D325`, Init
  `(W,H,5,16)`, stride `(2W+3)&~3`, draw system slot 4, context Init/DrawPreclippedSpanList/Shutdown), labelled
  "constructed buffer", distinct from a shipped producer.
- §8: `docs/specs/proofs/span565_bridge.py` (unicorn): entries, pops, registers, examples, slot 0 — **PASS**.
- §9 + `docs/specs/questions/span-render-565.md`: **SR565-5** (runtime: is `0x00826472` / `0x008307A0` ever hit),
  **SR565-6** (Init's type-3 exit and Release-without-Shutdown leave the destination locked: reproduce or not).
  QUESTIONS.md regenerated.

**Notes.** REA not used (static fallback: re_query.py, raw image dwords via `build-runs/raw.py`, unicorn). The
graphic-system `+0x24` pair used when QI `0xC47B747C` fails was not resolved to a type (covered by SR565-5).
No ABI rows, no engine/tools changes, no game runs.
