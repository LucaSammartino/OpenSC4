| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-particles4: particle component spec (cSC4ParticlesEffect family) | `re-particles4` / (RE spec lane) | ready: new spec docs/specs/particles.md (af2d5684), 12 sections; fully specified cSCParticleSimulatorBase (0x005B9xxx only), cSC4EffectsParticle, cSC4ParticlesDescription; cSC4ParticlesDescriptionBase specified (layout/defaults/file order, meanings partly open); cSC4ParticlesEffect, cTractorPoint, cWiggle partial; questions PT1-PT7; REA unavailable, export-only evidence | 1. Answer PT1/PT5/PT6 (InitEffectsParticle, Start tail, field names) to credit cSC4ParticlesEffect. 2. Apply the coverage proposal below at the next train. 3. Correct effects.md §7.1 item 2 class attribution (particles.md §8). |

## Report

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
