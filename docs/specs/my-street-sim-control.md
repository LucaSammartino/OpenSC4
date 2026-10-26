# The MyStreetSim picking control `cSC4MyStreetSimInputControl` (behaviour spec)

Scope: the view-input control with which the player picks the automaton (a car or a person) that the street sim
follows. The street sim itself and its `AttachAutomaton` (Street slot 3) are my-street-sim.md; the base control is
city-tools-input.md §2; the automaton interface is automata.md.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). All seven functions were read in the disassembly; the box constants were read in the PE.
Written 2026-10-25 (re-streetctl8).

## 1. State and lifetime *[compat]*

Constructor `0x0043E760`: the base constructor, vtable `0x00A85F40`, control id `+0x0C := 0xCC23D564`, `+0x28` (the
street sim's Street interface, held) and `+0x2C` (the automaton under the cursor, `cISC4Automaton` `0x8B99EE7C`,
held) := null. The deleting destructor (`0x0043E870`) releases `+0x2C` and `+0x28`, runs the base destructor and
frees on flag bit 0.

- **Init()** (`0x0043E740`, slot 3): `SetCursor(0xEBF9C962)`, then the base `Init`.
- **Init(sim)** (`0x0043E790`, non-virtual, one argument): when slot 3 returns true, hold `sim + 0x2B8` (the street sim's Street
  interface; null for a null sim) in `+0x28` (AddRef the new, Release the old, nothing when equal) and return true;
  otherwise false.
- **Shutdown** (`0x0043E7E0`): release and clear `+0x28`, the base `Shutdown`, true.

## 2. Hover (`OnMouseMove`, `0x0043E8B0`) *[compat]*

1. Release and clear `+0x2C`.
2. `PickOccupant(x, z, &o)` (3D view slot 37). On a hit: `a := o.QueryInterface(0x8B99EE7C)`; when that succeeds,
   `a.CheckAutomataFlag(1)` (slot 31) is false and o is in any of the groups `0x420B`, `0x4201`, `0x4215`, `0x4001`,
   `0x4200` (occupant slot 22, tested in that order), `+0x2C := a` (held).
3. When no automaton was taken: `PickTerrain(x, z, &p, 1)` with p preset to zero; on a hit, take the occupants whose
   boxes meet `p ± (3, 7, 3)` (occupant manager slot 37 `GetOccupantsByBBox(&list, min, max, 0, −1)`; the list nodes
   come from the allocator service cached in `0x00B207E4`). Walk them in order; the first whose automaton interface
   exists, whose flag 1 is clear and which is in group `0x4001` becomes `+0x2C`. Release the list (slot 48).
4. `SetCursor(0xEBF9C971)` with a target, `SetCursor(0xEBF9C962)` without. Release the local references; return true.

## 3. Click (`OnMouseDownL`, `0x0043E810`) *[compat]*

Only when this control is on top (3D view slot 24 is this control); otherwise false. With a target: read the
boolean property `0x1BC42C49` of the target's occupant (automaton slot 5 `AsOccupant`, its property holder, slot 6;
`cSCPropertyHelp::GetPropertyValue`); when that read **fails**, `+0x28` slot 3 (`AttachAutomaton(target)`,
my-street-sim.md §3.5). Returns true.

## 4. Conformance tests

1. Hovering a car in group `0x4200` with flag 1 clear sets the target and the cursor `0xEBF9C971`; a car with flag 1
   set leaves no target and the cursor `0xEBF9C962`.
2. Hovering empty ground with an automaton of group `0x4001` within 3 m horizontally takes it as the target; one of
   group `0x4200` at the same place is not taken by this fallback.
3. A click with a target whose occupant has a readable `0x1BC42C49` does nothing.
