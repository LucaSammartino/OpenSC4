tag: dbpf-ex56-save-514
run: 514
date: 2026-10-24
lane: glue-dbpf-ex56apply11
dll: FreeCity4-glue-dbpf-ex56apply11 (ec17f769; engine source e73dbdebb)
pacing: normal
verdict: clean
switches: disable=budget,ordinance; dbpf_shadow=1; autotest_unpause=2; autotest_end=quit; popup_dialogs=0; script=build-runs/dbpf-ex56-save.script; end_date=2032-01-01; seconds=1650; timeout=1800
result: dbpf 73/73; clean shutdown at 2032-01-02, real_seconds=236.328, wall=251 s, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0. Full SaveCity on 2029-06-15 succeeded in 11156 ms. Explicit city comparison ran four times before save (187 records) and four times after save/re-registration (153 records); save_generation advanced from 0 to 1. Eleven packed files: 10 roots and 1 multi-packed child; city discovered on all 73 samples, no fallback used. Initial full sweep 111826 records; final cumulative 84 files/876071 records; rotation cycle 6 covered 11/11 files and 111792/111792 records after save. Refusals=0, differences=0, max_slice_ms=921. Transient/unregistered files remain unobserved (EX57).
summary: EX56 DBPF normal-speed save/re-registration, dbpf 73/73 exact and clean to 2032

Evidence: build-runs/game-results/dbpf-ex56-save-493.verdict and .log; consumed request build-runs/game-queue/done/dbpf-ex56-save-493.req. Exact count line: `dbpf-ex56-save-493 73/73`. The backing path was C:/Users/tlori/FreeCity4/build-runs/sc4slots/slot2/Regions/FC4 Test/City - Konradshohe.sc4. PreSave releases pending acquired views; later monthly samples rediscover the registered backing file. This measures the stable file before and after the save, not transient files during the save.
