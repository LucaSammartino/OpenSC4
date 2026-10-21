tag: glue23-replace-1
run: 160
date: 2026-10-16
lane: glue23
dll: FreeCity4
pacing: turbo
verdict: clean
switches: enable = demolition, script = build-runs/glue23-demolish.script, turbo to 2032-01-01 on Konradshohe
result: **clean**, ref_warnings=0, rtc=0, overruns=0, 79.33 days/s. `subsystem 'demolition': enabled (1 class replaced) by FreeCity4.ini (enable=)`; `[v] AddCls(0x0990C38A) registered (cSC4Demolition, subsystem 'demolition')`; `cFC4Demolition created, core ...`; `call Demolition::Init (#1)`. Read ran before Init. The full 86-field host table is wired (84 callbacks).
summary: glue23 enable = demolition, clean, the replacement loads and initialises
