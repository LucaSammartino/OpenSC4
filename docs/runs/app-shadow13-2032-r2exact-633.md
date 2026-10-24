tag: app-shadow13-2032-r2exact-633
run: 633
date: 2026-10-24
lane: glue-app-shadow13
dll: FreeCity4-glue-app-shadow13 d0cad6574 (sha256 prefix 5df41eb8)
pacing: turbo
verdict: clean
switches: app_shadow=1, app_shadow_path_cache=0 (default), popup_dialogs=0, turbo=1, end_date=2032-01-01
result: Clean to 2032-01-20 in 28.188 real seconds; ref_warnings=0, foreign_exceptions=0, rtc=0, watchdog_stalls=0. app 72/72; app-startup 1/1; all supported shadow lines mismatches=0, 17 fields per sample, resyncs=0. This measures eight stored-path getters, six supported lazy folders, two filename getters and message-derived mode. AP17 leaves startup replay, pause/speed, app counters and idle/pump decisions open; AP18 slots65/66 remain excluded from these labels and their separate diagnostic stays default-off. No whole-owner verification claim.
