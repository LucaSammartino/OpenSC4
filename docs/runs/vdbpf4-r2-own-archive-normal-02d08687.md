tag: vdbpf4-r2-own-archive-normal-02d08687
date: 2026-10-22
lane: verify-dbpf-native4
dll: build-runs/dbpf-r2-frozen-02d08687/FreeCity4.dll (SHA256 3d1eee0c6a7d55b992484c71a7373def98edc30364431198ce649a0bf21966d4)
pacing: normal
verdict: PASS (clean)
switches: disable=budget; enable=clock24; dbpf_native=1; refaudit=1; sim_turbo=0; end_date=2032-01-01
result: clean2032-01-01, ref_warnings=0, foreign exceptions/stalls/RTC/overruns=0; natural60440/60440 and constructed10/10 record transports exact; original combined EX55 differences natural1/constructed2 preserved; count differences0; full save ok1/12719ms; PostSystemServiceShutdown reached.
summary: Public record adapter versus production Rust Package; original packed segment independently owned; empty-list status false and missing DIR enumeration key remain EX55. No cRZFile ownership claim.
