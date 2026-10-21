| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-particles4: particle/decal/visual effect component spec | `re-particles4` / (RE spec lane) | in progress | see Handoff |

## Handoff
- Goal: specify the largest coherent uncovered particle/decal/visual effect component owner family (exported instruction evidence), extending docs/specs/effects.md or a focused new spec.
- Done: guard preflight GO (2026-10-21).
- Family chosen: cSC4ParticlesEffect (18819 B) + cSC4ParticlesDescriptionBase(+cTractorPoint,cWiggle) + cSC4ParticlesDescription + cSCParticleSimulatorBase (0x005B9xxx only) + cSC4EffectsParticle. New spec docs/specs/particles.md.
- Next: write docs/specs/particles.md from facts below, then file questions PT1..
- Facts: REA down (no app open); export-only. Particle record readers 0x005B0B50 (section ver 1) / 0x005B1DC0 (else; adds +0x04 u32&0xFF and +0x1CC string list via 0x005B1D60); call site 0x005A52BB. Parser commands write description at parse+0xF0 (EndBlock 0x005A1A50); build-runs/pfields.py prints keyword->desc offsets.
- Classes: base ctor 0x005B2B50 (vtables main 0x00AA0EA0, +4 0x00AA0E58); derived ctor 0x005A8ED0 (0x00AA0B88) and 0x005AC510 (0x00AA0C58). Factory 0x00591748: copies=comp+0x4E; desc+0x08 bit0 clear -> 0x00590BC0 base; set -> desc+0x1D8 vs [0x00A81054]: equal -> 0x00590D40, else 0x00590EA0. Base slot 15 = 0x005B96E0 (effects.md 7.1 has the slot-15 attribution reversed).
- Many cSC4ParticlesEffect vtable slots (0x005B3820..0x005B6390, 0x005B4B90 6079 B) are owned by cSC4EffectsManager via reach in coverage; propose ranges.
- build-runs/dasm.sh (full asm dump+filter), vt.py (vtable dump).
