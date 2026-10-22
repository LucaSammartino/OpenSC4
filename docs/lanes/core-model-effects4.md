| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-model-effects4: model registration effects | Codex / `core-model-effects4` (engine lane) | in progress; detached reserved worktree | 1. Implement public model effects contract. 2. Focused checks and full gate. |

## Handoff
- Goal: implement cSC4ModelRegEffects (23 functions, 8406 bytes) as a core owner using public S3D REGP records.
- Done: initial handoff cabc8a57; implemented the documented Consumer 1 subset, owned effect list and acquired model attachment, axis/angle composition, controls, checked frame diagnostics and public transform flags.
- Done: 14 new core tests plus 1 real-parser integration test pass in focused quick cargo checks; native quick build still running (session 33185).
- Next: finish quick build; check formatting; stable full qbuild all; report partial-owner scope and queue wip.
- Questions: GF19-GF24 appended to docs/specs/questions/graphics-formats.md. Complete 23-function inventory/native lifetime/visibility/control contracts are absent from the specified public sections; no complete-owner or runtime credit.
- Dependency fact: formats depends on core; provider borrows Model.regions' exact public BTreeMap<String, Vec<[f32; 7]>> type, avoiding a crate cycle or duplicate parser.
- Scope: core only, no C ABI/plugin/maps/tools changes or runtime promotion; no subagents.
- Baseline: task says main freshly gated; skip repeated full baseline, run focused quick then one final stable all gate.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-model-effects4-r1-manifest.json.
- No game requests outstanding.
