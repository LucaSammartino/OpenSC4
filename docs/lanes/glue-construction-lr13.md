| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-construction-lr13: construction reference verification | Codex / glue-construction-lr13 | in progress | Implement LR13, gate, measure house shadows to 2032 |

## Handoff
- Goal: LR13 construction reference codec/replay, T66-T73, complete defined-field comparisons.
- Done: initial handoff 7af10565c; LR13 literal-IID acquisition/release and canonical Serializable mapping implemented.
- Done: T66-T73 native DLL fixture added; CAPI rejection/continuation test passes (1/1).
- Done: construction codec preserves rejected entries as null even when the helper returns false without stream error; full record equality remains strict.
- Next: collect focused core test session 35931, format check, then qbuild all and queued verify on final source.
- Focused test logs: first CAPI test passed; core t22_visual_effect_record_quirk session 35931 is outstanding.
- Failed tooling evidence: fmt relative paths rejected; q.py Bash could not CreateFileMapping; direct PowerShell cargo works.
- Comparison scope: all 15 serialized fields, inherited holder and all reference identities/membership; crane presence explicitly seeded from captured bit, no lifecycle claim.
- Required: qbuild all plus queued verify; house profile to 2032, unique request label, collect same operation.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-construction-lr13-r1-manifest.json.
- No game request outstanding. No source edits yet.
