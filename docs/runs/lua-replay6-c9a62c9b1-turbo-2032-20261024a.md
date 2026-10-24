tag: lua-replay6-c9a62c9b1-turbo-2032-20261024a
run: 382
date: 2026-10-24
lane: glue-lua-replay6
dll: 2D4B35923B7B650C9DBE1D3DC3111EAC6FCE645C40759C26FAA74EED48F1A758 (runtime source c9a62c9b1)
pacing: turbo
verdict: clean
switches: lua_shadow=1, sim_turbo=1, popup_dialogs=0, autotest_end=quit; Konradshohe, end_date=2032-01-01, seconds=240, timeout=600
result: clean at 2032-01-21; ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0. lua 73/73 (house summary and independent dated-line count), all 47 entries per sample and all collector/binding/scratch/stack/capture differences zero, 3431 per-entry observations. First sample 2026-01-27 load; last 2032-01-01 month. Scratch=0x0040C890. City 29.969 s, queue wait 7 s, wall 40 s. Bounded registration state only; no full-VM verification credit or nondeterminism exclusions. Collected the unique verdict with one file-event wait, without qwait; no requests outstanding.
