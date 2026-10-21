| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-timer-core-answers4 (RE: answer timers TM2, then TM1) | `re-timer-core-answers4` / `.claude/worktrees/re-timer-core-answers4` | in progress | answer TM2 into timers.md §1.3/§2.2; TM1 if tractable |

## Handoff

- Goal: answer TM2 (a-d + UseTimerService) and TM1 in docs/specs/timers.md; move to answered/timers.md; questions.py index.
- Done: asm read (static, re_query.py). Findings TM2: (a) unit0 now = cpuid+rdtsc in all three GetElapsedTime (0x008901c5, 0x008902a7, 0x008903a4) MATCHES;
  (b) FILD qword (signed) 0x00890233 etc, compare unsigned JC/JA/JBE 0x008901ec MATCHES; (c) StartOnce stores neither (0x00932d32..3b writes only the stack arg) MATCHES;
  (d) failure clears +0x18 and +0x19 (0x00932d77/7a) MATCHES; UseTimerService no change -> AL=1 (0x00932af6) MATCHES; ctor writes +4:=1 then +4:=0 at 0x00932c8d before direct call MATCHES.
  Extra: UseTimerService keeps result of looked-up object's slot 3 (0x00932aa2), releases looked-up obj; false path writes default without releasing old (0x00932aef, leak).
- Next: write the rules into timers.md, then TM1 (default obj 0x009335F4 vtable slot 3).
