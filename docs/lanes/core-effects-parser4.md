| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-parser4: bounded portable parser | Codex / core-effects-parser4 | in progress | Literal tests, owner implementation, stable gate |

## Handoff
- Goal: first portable effects_parser owner; no native/ABI/plugin work.
- Start HEAD: 5fa5dbe8147531e9a8dafca083602370f820c7fe (detached, reserved).
- Read rules, contribution rules, parser sections 1–8 and questions EFP1–6.
- Existing core-effects4 explicitly excludes parser; no core parser symbol found.
- Transcribed P1–8/R1–6/M1–4/Q1 as 19 independent portable fixture groups before production implementation.
- Implemented option order/prefixes, local append, logical references/QI and ordered registrations.
- Reuses ComponentRecord; type_word retains full +0C; placement/LOD shared-constructor projection differs from shipped defaults.
- Next: focused tests, additional unsupported/numeric/registration cases, then commit stable source and canonical all.
- Numeric scope: finite exact dyadic input, nearest binary32 stores; broader x87 unsupported.
- No game request; portable tests only. EFP1–6 remain open.
- Brief overrides general baseline/quick requirement: one canonical all on stable committed source.
