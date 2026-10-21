| re-construction-answers4 (OP7 + OP13-OP16 answers, lot-records §9 / lot-developer §6.7) | `re-construction-answers4` / spec lane | ready + report (5 questions answered, all DIFFERS; spec text only, no engine/ABI change) | core lane: re-key `ConstructionScalars` to the §9.1 object-frame roles, resume the schedule from loaded #14 values, use 4 subscriptions + base Init + AddOccupantGroup(0x5007), sparks x/z extent, crane limits/T/k per ld §6.7.6 |

## Report

**Key finding.** `cSC4ConstructionOccupant` methods run on four `this` frames: object (+0: SetLot, DoDailySim,
DoAnimationTick, AnimateCrane, RebuildModel, CreateSparks), +4 (DoMessage), +8 (occupant: Init, Shutdown,
GetPosition, GetBoundingBox), +0xC (Write/Read). The old lr §9.1 table used Write-coded offsets (12 low), which
produced every OP13/OP14 contradiction. lr §9.1 is now in the object frame with the Write offset in brackets.

**What changed**
- `docs/specs/lot-records.md` §9.1: frame table; full #3-#15 field table with roles (state s8, T0, unused u32,
  nextDay, N, counter, span, startY, A, B, C, E, msAcc, crane state s8, swing min/max, unused bool, crane acc/duration,
  target/previous angle, crane type u8); #3/#4 are the live refs; #8 written from crane pointer +0x104, read into
  load-only +0xD8; effect vector entries written as pointer + 4 (was "+0x10").
- §9.2: Init order (IsInitialized, base Init unconditionally, StaticInit, 4 subscriptions, tick 0x80, grids, render
  state, AddOccupantGroup(0x5007) with its proved body 0x005E9350); DoMessage (tick ms = msg dword +0x14 / 1000);
  refresh-after-load path (+0xD9 pending, CreateCrane from saved pos/type, state 2→1→2 around DoAnimationTick(0));
  SetLot crane init; GetPosition reads record #6 box; life cycle resumes from saved #14.
- `docs/specs/lot-developer.md` §6.7.1 Init/tick, §6.7.6 AnimateCrane (duration = round_f32(T·|Δ|·k), T = tunable
  sfCraneTimeOfFullRotation 0x00B0A7EC, k = 1/(2π) f32 0x00AA2098; initial target/prev/acc/duration; swing limits
  c ± h from AttemptAddOuterCrane), §6.7.7 sparks extent = max(x, z span) of scaffold box (y excluded).
- Questions OP7, OP13, OP14, OP15, OP16 moved to `docs/specs/questions/answered/occupants-props.md` (new), all
  DIFFERS; QUESTIONS.md regenerated.

**Evidence**: static only (re_query asm/vtable, decomp/); REA not used. Constants read from bin/SimCity4.exe
(build-runs/peval.py). Not proved: what `FUN_005CBE10` does beyond posing the boom; the message setters of 0xEBFF18A3
by name; the y extent of the scaffold box.

**Next**: core lane applies the contracts above (no ABI rows proposed).
