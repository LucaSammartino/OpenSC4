# Growth developer: open questions

- **GD-GC29-2 — state-18 iterator native ABI (2026-10-26, growth-cities29):**
  Section 5.2.1 supplies the successful-pair capture boundary, but simgrid §7.4 describes
  `GetNext(&x, &z)` as boolean while `docs/abi/functions.tsv` classifies `0x0060C8D0`
  as `edx:eax`, thiscall, two stack words. Please reconcile the success value in
  the public interface contract (ABI facts belong in docs/abi), and supply
  value-only fixtures: each delivery's ordered successful coordinate pairs,
  terminal result, input residential cell values and expected tract populations.
  No native hook or implementation excerpt is requested; a zero coordinate must
  remain distinguishable from an unsuccessful iteration.
  Blocks state-18 same-input capture in `growth-cities29-12/lua-lots-grown.log:3348`:
  population differs in seven residential tracts (maximum 844), with D, capacity and
  maximum capacity exact. The clean late-date gen27-Konradshohe pass is 301/301 exact;
  it does not establish the unrecorded Lua per-tract inputs. Evidence root:
  `C:/Users/tlori/FreeCity4/build-runs/suite/`. No formula or tolerance changed.
