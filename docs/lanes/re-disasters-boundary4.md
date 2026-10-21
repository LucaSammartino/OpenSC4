| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-disasters-boundary4: answer DS42-DS44 (disaster layer replacement boundary) | `re-disasters-boundary4` / .claude/worktrees/re-disasters-boundary4 | ready, docs only, no ABI change; DS42 MATCHES, DS43 DIFFERS, DS44 MATCHES (disasters.md §2.8, tests 23-24) | 1. glue: `enable = disasters` may register a factory for `0x61F6ABF5` per §2.8 contract, keeping the native managers. 2. glue: reference-count conformance tests 23-24. |

## Report

**What changed** (commit a498bf17): docs/specs/disasters.md has a new §2.8 "Replacement boundary", §1 now links to
it, and §18 has conformance tests 23 and 24. DS42-DS44 moved to questions/answered/disasters.md, and QUESTIONS.md
was re-indexed.

**Findings** (all static: raw byte scans of bin/SimCity4.exe, re_query asm and capstone windows; REA was not needed):

- **DS42.** City Init calls `CreateCityComponents`. That function calls `GZCOM GetClassObject(0x61F6ABF5,
  0x29B02A2C, &city+0x1C4)` at `0x45B004`, only if the slot is null.
  - Factory `0x578B70`, registered by the COM director at `0x460B57`. It allocates 0xD8 bytes and returns `+8`
    with no AddRef.
  - The city holds 1 reference. The layer is not saved: city Read/Write use `this` = city+8.
  - The layer `Init` runs after the civic building simulator and before demolition.
  - City `Shutdown`: it clears the global first, then shuts down the later components, then calls layer
    `Shutdown` (slot 4) and `Release`, then the civic building simulator.
- **DS43.** `SL::OnCityInit` (`0x601D79`) writes the global from city slot 72 without AddRef. This happens before
  the layer `Init`. `SL::OnCityShutdown` (`0x602056`) clears it.
  - 8 reader instructions in 7 functions, calling slots 7, 10, 16 and 17. Only the tutorial null-checks.
  - The effect (`+0x20`) and toxic (`+0x14`) instances take city slot 72 and AddRef in `Init`. Their `Shutdown`
    calls slots 9 and 14, then `Release`.
- **DS44.** I audited all 119 call sites through slot 72. Nothing outside the layer QIs it, and the only raw read
  is the slot 18 record. Six instance `Init`s and plumbing `ApplyPipeFunding` cast the slot 7 result to the
  concrete native manager (−8). Slot 7 must therefore return the native managers' `0x021E9CBF` pointers.

**Unchecked:** whether the message server AddRefs the layer's `+0xC` subscriptions. This does not matter for the
contract, because the layer unsubscribes in `Shutdown`. No new question was filed.

**Helper scripts** (lane-local, not for merge): build-runs/dsb-scan.py, flow.py, win.py, back.py, mgrcast.py;
audit output in build-runs/audit120.txt.
