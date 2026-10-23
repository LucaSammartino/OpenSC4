tag: vdbpf4-r2-current40-save-on-02d08687
run: 279
date: 2026-10-22
lane: verify-dbpf-native4
dll: build-runs/dbpf-r2-frozen-02d08687/FreeCity4.dll (SHA256 3d1eee0c6a7d55b992484c71a7373def98edc30364431198ce649a0bf21966d4)
pacing: turbo
verdict: WIP (harness clean, ref_warnings4)
switches: disable=budget; enable=literal coordinator HOUSE40 list; dbpf_native=1; refaudit=1; fixture_city=commands4-r2-house40.sc4; dbpf_native_file=same immutable fixture; end_date=2035-01-01; full request retained in game-queue/done
result: clean2035-01-01 with ref_warnings=4 (not ref0), foreign exceptions/stalls/RTC/overruns=0; saved153/153 record transports exact (compressed80); constructed10/10; EX55 combined differences1+2; count differences0; failed0/refusals0/fallback0/resync0; full save ok1/11125ms; PostSystemServiceShutdown reached.
summary: Supplied saved-city fixture 4AD4E47B96E66110216CE0A2794C7F0CD6CDB5450295B97D320E7E94469EFFF8. Four last-reference warnings (class0xE990BFFC):2 at DropObject:144,1 at DropObject:126,1 at ReleaseToken:465. Retained; follow with verifier-off control, no clean/ref0 promotion.
