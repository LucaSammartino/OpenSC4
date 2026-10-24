tag: lotrec-r3-triple-2032-381-389
run: 389
date: 2026-10-24
lane: glue-simgrid-lotrec5
dll: 97b468413 / FreeCity4.dll sha256 prefix f1ecb12d
pacing: turbo
verdict: clean
switches: automata_shadow=1, simgrid_shadow=1, lotrec_shadow=1, popup_dialogs=0, close_dialogs=1; Konradshohe to 2032-01-01
result: clean to 2032-01-01, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0; every harness problem count zero; real_seconds=42.813, wall=54 s. Automata 73/73 observations exact, 438 components and 786414 bytes compared, mismatches 0. Simgrid 73/73 observations exact; final roles=86 expected=86 developers=12 exact=86/86 mismatches=0 differences=0 unavailable=0 cells=311552 bytes=423538. Lotrec 73/73 observations exact; final expected=672 visited=672 captures=672 exact=672/672 mismatches=0 differences=0 errors=0 empty=0 bytes=241893. Unique completed verdict retained without resubmission.
summary: Round 3 shared native variant encoder; automata, SG3 and LR12 exact through 2032
