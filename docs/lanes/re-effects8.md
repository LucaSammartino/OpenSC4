| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects8: the effects owners effects.md, particles.md and effects-parser.md left open (model-based and dynamic particles, camera effect, effect placement tool, deformer, resource factory), effects-owners.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-tsound8) | **ready** (docs only). Twenty-two owners claimed: specified +19,840 bytes (+95 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Every function was read in the decompile; the renderer, the fake perspective, the deformer and the camera helpers
also in the disassembly. Constants and the draw-option enum table were read in the PE. Before writing, every owner's
functions were checked against the existing specs with or without the `0x` prefix, so bodies effects.md already
cites were not specified twice. Written on 2026-10-25.

### What changed

- docs/specs/effects-owners.md (new): §1 the model-based particles effect (members, `ApplyFakePerspective`,
  `RenderActiveParticles`); §2 the dynamic particle effect; §3 the camera description, effect and parameters; §4 the
  effect placement tool, including the terrain-controlled effect state slot; §5 the dynamic description destructors;
  §6 `DeformModel`; §7 the resource factory; §8 the leftovers tools/coverage.py assigns to `cSCParticleSimulatorBase`;
  §9 the owners that were already specified (decal description, light, decal, shake and group commands, description
  record, the three parser helpers with the draw-option enum); §10 quirks; §11 conformance tests.

### Coverage

- tools/coverage-map.tsv: new rows for the model-based and dynamic particle effects, camera effect, description and
  parameters, effect placement tool, dynamic particle description, simulator base, effects model processor, resource
  factory, decal description, description record and the three parser helpers, each before the catch-all it would
  otherwise fall into; the four anonymous-namespace command rows filled.
- Specified bytes 5,061,522 → 5,081,362 (+19,840, 91.0%, `cov_local.py` with owners-c1).
