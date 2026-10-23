tag: vdbpf4-r2-own-save-off-normal-02d08687
run: 278
date: 2026-10-22
lane: verify-dbpf-native4
dll: build-runs/dbpf-r2-frozen-02d08687/FreeCity4.dll (SHA256 3d1eee0c6a7d55b992484c71a7373def98edc30364431198ce649a0bf21966d4)
pacing: normal
verdict: PASS (clean)
switches: disable=budget; enable=clock24; dbpf_native=0; refaudit=1; sim_turbo=0; fixture_city=commands4-own-e50c5bbc.sc4; end_date=2035-01-01
result: clean2035-01-01, ref_warnings=0, foreign exceptions/stalls/RTC/overruns=0, verifier lines0; full save ok1/11172ms; PostSystemServiceShutdown reached. Same immutable fixture and DLL/PDB as failed turbo control; only pacing changed.
summary: Supported own saved-city reload verifier-off ref0 control. The earlier turbo control exited code1 and remains preserved without a clean claim or assigned cause.
