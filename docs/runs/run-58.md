tag: run-58
run: 58
date: unknown
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: `clock_probe = 2` (week agent held once), our clock, plumbing + flammability; posted `WM_CLOSE` at 152 s
result: **clean.** Mode 2 shows the §5.3 repeat: every week message is delivered twice (`… again`). The posted `WM_CLOSE` did nothing (no dialog, the city kept running; with `popup_dialogs = 0` a quit prompt would answer by itself), so scripted quitting moved into the plugin: `autotest_end = quit`.
