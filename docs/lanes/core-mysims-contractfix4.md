# core-mysims-contractfix4 — correct MySim dispatch/pool cores to answered MS11/MS12

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-mysims-contractfix4 (correct mysims dispatch §4.7.1 / pool §4.12 cores to MS11/MS12 answers) | `core-mysims-contractfix4` / `.claude/worktrees/core-mysims-contractfix4` | in progress | see Handoff |

## Handoff
- Goal: dispatch neighbor=-1, IsValid (non-ferry/city bounds, no occupant check), SetByOccupant clears byCommute,
  trip-state field, job-search notifications; pool import lengths mod 256, UserData portrait path. Core +
  conformance only (no plugin/CAPI).
- Done: 06cee36b dispatch location contract (MS11 a-e), tests t30-t41, vectors ms12_21..25 migrated + ms12_28,
  question MS13 filed in docs/specs/questions/mysims.md.
- Next: pool.rs per MS12: encode_record len mod 256; picture folder <ud>\Web Pages\<v>\FamilyGFX\; F.ini =
  Export dir + Left(len-4) + .ini; ini case-insensitive; ReadNumber %ld (miss -> None); nummembers unsigned loop;
  absent template props explicit (indeterminate marker). Then qbuild all, report.
- Facts: base main d68b254a. Baseline cargo test 3980 passed, clippy 0. Spec §4.7.1 = mysims.md 392-543, §4.12 =
  635-781. sim.rs FailCommute still dispatch_command("reset_commute") (MySim side, remainder).
