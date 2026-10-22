tag: vdbpf4-r2-own-save-on-02d08687
date: 2026-10-22
lane: verify-dbpf-native4
dll: build-runs/dbpf-r2-frozen-02d08687/FreeCity4.dll (SHA256 3d1eee0c6a7d55b992484c71a7373def98edc30364431198ce649a0bf21966d4)
pacing: turbo
verdict: PASS (clean)
switches: disable=budget; enable=clock24; dbpf_native=1; refaudit=1; fixture_city=commands4-own-e50c5bbc.sc4; dbpf_native_file=same immutable fixture; end_date=2035-01-01
result: clean2035-01-12, ref_warnings=0, foreign exceptions/stalls/RTC/overruns=0; saved fixture153/153 record transports exact (compressed80); constructed10/10; EX55 combined differences1+2; count differences0; failed0/refusals0/fallback0/resync0; full save ok1/11500ms; PostSystemServiceShutdown reached.
summary: Reload of supplied saved-city fixture FF38A466B52443575DFC03761F51702169EA996020721B9FE124268300092118 through documented fixture_city route. This is not a lane-produced save roundtrip. Public record adapter only; original packed owner.
