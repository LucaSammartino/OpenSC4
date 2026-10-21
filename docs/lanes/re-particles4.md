| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-particles4: particle component spec (cSC4ParticlesEffect family) | `re-particles4` / (RE spec lane) | in progress (round 3 from 5297a6c1): particles.md §5.1 InitEffectsParticle, §6 Start complete, §6.1 SetDescription inputs, §8/§8.1 emitter proof+dispatch; effects.md §7.1 item 2 corrected; PT5/PT6/PT7 answered, PT1 narrowed; cSC4ParticlesEffect still partial (physics, rates, drawing) | 1. Physics 0x005B9BA0/0x005B9790 + emitters 0x005B3CF0/0x005B8B50/0x005B87B0 to finish cSC4ParticlesEffect. 2. Coverage rows per report (train owner). 3. PT1 rest, PT2-PT4. |


## Handoff (round 3, 2026-10-21)

Goal: round 3/3 (final): specify physics 0x005B9BA0/0x005B9790, emitters 0x005B3CF0/0x005B8B50/0x005B87B0, draw
0x005B3DA0/0x005B4B90, parsers 0x0059C280/0x0059A820; finish PT1, PT2-PT4 as far as bytes support. Keep Init/Start/
identity corrections. cSC4ParticlesEffect uncredited until core fully specified; credit only range 0x005B9CC0-0x005B9FB7.
Done: rounds 1-2 (reports below). Round 3: 5c82f711 §7.1 physics (both paths, exact); 9a53e220 §8.2 three emitters
(exact), §8.3 age events (PT2 run-time reader 0x005B84C0/0x005B8410).
Next: drawing 0x005B3DA0/0x005B4B90, then parsers 0x0059C280/0x0059A820 for PT1 keyword names; PT3/PT4; update §10, PT1/PT2.
Facts: REA unavailable this round (no app open) - export+decomp only; helper build-runs/pe.py reads exe constants.

## Report (round 2, 2026-10-21)

**Changed.** docs/specs/particles.md (b59b3c3a, 688f34c9): new §5.1 (InitEffectsParticle 0x005B7580: every field,
draw order, 8 vs 14/15 draws, transforms, quirks), §6 steps 4-7 (draw order, preferSea via PositionToCell /
GetSeaportDeveloper / GetCellSeaAccess and the +0xC8 frame, countdown +0x190 from +0x20 or +0x28, mode/preroll
byte), §6.1 (SetDescription 0x005B7C90 inputs: emission box +0x200 from desc +0x50, preferDir, collide names),
§8 vtable-word proof, §8.1 dispatch 0x005B96E0 (countdown, preroll−0.01 cap, flags A 0x4/0x2), §2 meanings for
13 fields, flags A 0x20. effects.md §7.1 item 2 rewritten (behaviour contract unchanged: same three emitters, same
Stop(0) sites; only class/vtable identity fixed). Questions: PT5, PT6, PT7 answered (answered/particles.md),
PT1 narrowed; QUESTIONS.md regenerated.

**Evidence.** Export + decomp only (REA not used). Numbers from instructions: constants 0.0 0x00A81054,
1e-4 0x00AA0D00, 1.0 0x00A81228/0x00B0A664, 0.5 0x00A84D2C, 1/(2π) 0x00B0A660, 0.01 0x00A92688; mask 0x7C000
from static init 0x00A73060; RandomDoubleRangeUniform 0x00913761 = lo + u(hi−lo), generator 0x00B22DF8.
Slot proofs: data words 0x00AA0EDC, 0x00AA0C94 → 0x005B96E0; 0x00AA0BC4 → 0x005AA5D0; 0x00AA0E58 slot 15 =
0x005B7C20.

**Exact owners vs gaps.** cSC4ParticlesEffect: Start 0x005B3000 (1,062 B), InitEffectsParticle 0x005B7580
(1,683 B), dispatch 0x005B96E0 (175 B) now fully specified; SetDescription 0x005B7C90 partial (§6.1). Still open:
physics 0x005B9BA0/0x005B9790, terrain 0x005B8DC0, collision 0x005B8020, tractor 0x005B6600, attractor
0x005B6C40, DoHeightBasedEmit 0x005B71E0, city volume 0x005B7030, the three emitters, drawing 0x005B3DA0/0x005B4B90.
So cSC4ParticlesEffect stays **partial**: do not map it yet.

**cSCParticleSimulatorBase owner rows (tools/coverage.py --owners, this tip): 23 functions, 1,703 B.** Particle
code (credit with particles.md): 0x005B9CC0 33, 0x005B9CF0 34, 0x005B9D40 59, 0x005B9D80 10, 0x005B9D90 24,
0x005B9DD0 19, 0x005B9E20 76, 0x005B9E70 136, 0x005B9F50 39, 0x005B9F80 55 = **485 B**. Not particle code (13
functions, 1,218 B; do not credit): 0x0048C920 23 (FUN_, vtable words 0x00A8ED64/0x00ABFCB0), 0x0048C940 28
(QueryInterface, shared: 7 vtable words incl. 0x00A8ED5C), 0x0048CDE0 299 (FUN_, only from 0x0048DAC0),
0x0048DAC0 30 (destructor name, vtable 0x00A8ED68), 0x004BDF00 8 (AddRef name; shared stub, 65+ callers, 96 data
refs), 0x00724BF0 96 (FUN_, only from 0x00725130), 0x00725130 59 (vtable 0x00AB3504), 0x00811CB0 28 (5 vtable
words), and the chain 0x0081CBA0 99 ← 0x0081CCD0 101 ← 0x0081CD40 81 ← 0x0081CEE0 336 ← 0x0081D030 30 (vtable
0x00ABFCB4/0x00ABFCBC). Any future row must be an address range 0x005B9CC0-0x005B9FB7, not the class name.

## Report (round 1)

**Scope chosen.** The uncovered "effects (particles, decals, visuals)" subsystem is 71,568 bytes (tools/coverage.py
--owners, 2026-10-21, base ab23a7fa). The largest coherent family is the particle component: cSC4ParticlesEffect
18,819 B, cSC4ParticlesDescriptionBase 3,149 B (+cTractorPoint 64, cWiggle 57), cSCParticleSimulatorBase 1,703 B,
cSC4EffectsParticle 46 B, cSC4ParticlesDescription 41 B. EffectsManager/Resource/brush/scrubber work was not
repeated; effects.md is referenced, not edited.

**What changed.** New docs/specs/particles.md: identity of the three particle classes (constructors, main and +4
vtables, the factory choice by flags B bit 0 and desc +0x1D8); the particle description record on disk (readers A
0x005B0B50 and B 0x005B1DC0 by section version), in-memory layout, constructor defaults and keyword names; flag
words; the simulator base protocol (Start, Stop(delayed, release_when_done), CheckForStop, Release/FinalRelease,
Add/RemoveParticle); particle free-list pools and the dead-store quirk; Start(mode) restart filter and random-walk
setup; the frame update order (slot 9); slot tables for 0x00AA0EA0 and 0x00AA0E58; a correction to effects.md §7.1
(emitter 0x005AA5D0 is the geometry-sourced class's; plain and model-based share 0x005B96E0). Questions PT1-PT7 in
docs/specs/questions/particles.md; QUESTIONS.md regenerated.

**Fully specified owners:** cSCParticleSimulatorBase functions 0x005B9CC0-0x005B9FB7 (~485 B), cSC4EffectsParticle
(46 B), cSC4ParticlesDescription (41 B). **Specified with open meanings:** cSC4ParticlesDescriptionBase constructor
0x005B0FE0 and copy 0x005B17F0 (2,512 B; reach functions 0x0059EB20, 0x005B0450, 0x005B1680 not examined).
**Partial:** cSC4ParticlesEffect (identity, Stop, Start head, Update order, CreateParticle/DestroyParticle, slots;
not physics, emission rates, InitEffectsParticle, SetDescription, drawing), cTractorPoint, cWiggle (element shape).

**Coverage proposal (tools/coverage-map.tsv, for the train owner; no tools edit made).** Rows with spec
particles.md placed before the `cSC4*Effect` wildcard: `cSCParticleSimulatorBase`, `cSC4EffectsParticle`,
`cSC4ParticlesDescription`, `cSC4ParticlesDescriptionBase`. Do not map `cSC4ParticlesEffect` yet (partial).
Caveat: the owner `cSCParticleSimulatorBase` also collects 0x0048C920/0x0048C940/0x0048CDE0/0x0048DAC0/0x004BDF00/
0x00724BF0/0x00725130/0x00811CB0/0x0081CBA0..0x0081D030 (~1.2 KB) by shared/ICF names; those are not particle code
and should not be credited by this row. Separately, 0x005B2B50 (constructor), 0x005B3820-0x005B6390 (slots 11-19,
incl. 0x005B4B90 6,079 B), 0x005B96E0 and the emitters 0x005B3CF0/0x005B87B0/0x005B8B50 are owned by
cSC4EffectsManager through the reach rule; they are cSC4ParticlesEffect code (vtable 0x00AA0EA0) and a range row
would move them.

**Evidence.** REA: no app open (batch_decompile target_unavailable), so export and decomp/ only; numeric claims from
instructions (factory compare 0x00591789, CreateParticle 0x005B9E33-0x005B9E60, readers).

**Next.** PT1, PT5, PT6 against InitEffectsParticle 0x005B7580 and Start 0x005B3000 tail would make
cSC4ParticlesEffect specifiable; then physics 0x005B9BA0/0x005B9790.
