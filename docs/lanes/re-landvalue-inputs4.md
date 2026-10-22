| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-landvalue-inputs4: LV1 native pass input capture timing and startup desirability snapshot | spec writer (RE), `.claude/worktrees/re-landvalue-inputs4` | ready: landvalue.md §6.1 specifies per-state input reads, the exact capture boundary (each land value Step call), a typed input record and the pass-1 startup snapshot; LV1 answered DIFFERS; a1 p2 explained structurally, not proved; docs only, no ABI | shadow implementer: capture per Step call (§6.1) or snapshot developer maps at load for pass 1 |

## Report

### What changed (fcee0bf6)
- `docs/specs/landvalue.md` new §6.1 "When each input is read" (from `DoIdle` `0x006BE7B0`, `DoMessage` `0x006BF0D0`,
  iterator `0x0060C8D0`, `SetupAltitudeEffect` `0x006BD750`): per-state read table (land/water at states 1, 3, 8;
  altitude at 3 and 4; zoning at 3 and 8; slope vertices at 5; developer map in each state-7 call; terrain limits at
  2 and end of 3); the budget test precedes `GetNext` (`0x006BE865`), so a call reads exactly the tracts it takes;
  object offsets (pipelineState `+0x38`, tract iterator `+0x3C`, slope iterator `+0x40`, iterator layout, developer
  cursor `+0x70`, ignoreZoning `+0x4C`); the faithful capture boundary (before/after each SimIdle/SimNewDay delivery
  to land value); a typed per-pass input record; startup rule with a worked example.
- §15 item 4 resolved. LV1 moved to `docs/specs/questions/answered/landvalue.md` (DIFFERS); QUESTIONS.md reindexed.

### Findings
- The original keeps no input snapshot; inputs are read per state and per call, and a pass spans frames and days.
- Pass 1 after a load: the BDS level-0 SimBegin starts every developer's update at once; land value reaches state 7
  at its 8th call, a developer state 17 at its 17th at the earliest. vlv49-a1/n1 p1 (intrinsic exact, total 391/383
  off with the live map) fit "pass 1 used the restored map; the developer published after". Previous-map candidate
  for pass 1 = any D snapshot after the load while slot 10 has stayed true.
- Unmatched a1 p2 (no SimNewDay reproduces; closest day 1, 3 intrinsic tracts off): consistent with states 1/3/4 of
  one pass reading different worlds; not provable from the log (no per-call capture, no zoning/terrain event lines).
  a1 p2/p3 flip the same tracts ((4,14) 8/7 then 7/8) and swap meanAlt 270.05667/270.05804: inputs went A, B, A.
- REA not used: static analysis from decomp/ and re_query.py asm.

### Questions
None filed; LV1 answered.

### Next steps
- Shadow lane: per-Step-call capture (§6.1) would prove or refute a1 p2; a load-time developer snapshot fixes p1.
