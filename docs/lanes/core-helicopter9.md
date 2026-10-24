| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-helicopter9: Rust cSC4Helicopter | Codex / core-helicopter9 | in progress | Map owner contracts, implement core and conformance tests |

## Handoff
- Goal: close cSC4Helicopter owner behavior from aircraft.md and automata.md; core only, no ABI/plugin edits.
- Done: reused reserved detached worktree; initial tree clean.
- Done: mapped aircraft sections 2/4/8 and automata sections 6.2/6.4/6.6/14; found legacy owner in automata/aircraft.rs.
- Next: move owner to helicopter/, implement AF1-resolved 13-field overlay and default goal pop; retain compatibility re-export.
- Facts: AF2–AF6 already cover unsettled rotor/AI/lifecycle/player/messages contracts; no new flight model may be invented. Preset defaults are absent from spec.
- Checks: final qbuild all and queued verify required; no game request outstanding.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-helicopter9-r1-manifest.json.
