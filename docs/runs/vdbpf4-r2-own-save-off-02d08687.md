tag: vdbpf4-r2-own-save-off-02d08687
date: 2026-10-22
lane: verify-dbpf-native4
dll: build-runs/dbpf-r2-frozen-02d08687/FreeCity4.dll (SHA256 3d1eee0c6a7d55b992484c71a7373def98edc30364431198ce649a0bf21966d4)
pacing: turbo
verdict: FAIL (exited)
switches: disable=budget; enable=clock24; dbpf_native=0; refaudit=1; fixture_city=commands4-own-e50c5bbc.sc4; end_date=2035-01-01
result: exited code1 without a verdict, wall58s, last logged day2033-11-15; harness summary reference warnings0/foreign exceptions0/structured exceptions0; no full-save or shutdown completion; native verifier lines0. This is not a clean/ref0 control.
summary: Preserve failed attempt unchanged. Follow with unique normal-speed control using the same immutable DLL/PDB and saved fixture; do not repeat the unchanged turbo setup.
