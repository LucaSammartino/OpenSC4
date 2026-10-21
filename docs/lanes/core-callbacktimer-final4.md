| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-callbacktimer-final4 (engine: timers.md §2.2 `cRZCallbackTimer` exact UseTimerService / service ownership) | `core-callbacktimer-final4` / (engine lane) | in progress | - |

## Handoff
- Goal: exact slot-11 UseTimerService (retain slot-3 scheduler, release system service, false path no release = quirk 8), typed service inputs, audit all slots; core only.
- Done: code a0aacdb5 (slot 11 exact, typed FrameworkTimerService/ServiceLocator, TimerService add_ref/release, remove->bool, dtor releases +0x20, slot-6 null object keeps +0x24; 3 new tests); TM3 filed b39438dc (QUESTIONS.md index not regenerated, tools/ off-limits). quick: cargo -p fc4-core 2254/0.
- Next: `qbuild all` (baseline ran concurrently), then finish report + merge-queue json. No game run (pure core, no glue).
