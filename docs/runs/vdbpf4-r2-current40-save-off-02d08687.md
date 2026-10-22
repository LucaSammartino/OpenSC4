tag: vdbpf4-r2-current40-save-off-02d08687
date: 2026-10-22
lane: verify-dbpf-native4
dll: build-runs/dbpf-r2-frozen-02d08687/FreeCity4.dll (SHA256 3d1eee0c6a7d55b992484c71a7373def98edc30364431198ce649a0bf21966d4)
pacing: turbo
verdict: WIP (harness clean, ref_warnings4)
switches: disable=budget; enable=literal coordinator HOUSE40 list; dbpf_native=0; refaudit=1; fixture_city=commands4-r2-house40.sc4; end_date=2035-01-01; full request retained in game-queue/done
result: clean2035-01-01 with ref_warnings=4 (not ref0), foreign exceptions/stalls/RTC/overruns=0, verifier lines0; full save ok1/11063ms; PostSystemServiceShutdown reached. Same immutable DLL/PDB and fixture as current40 verifier-on.
summary: Same4 last-reference warnings as verifier-on, class0xE990BFFC (public SDK DepartmentBudget):2 PortDeveloper DropObject:144,1 Parks DropObject:126,1 Landfill ReleaseToken:465. This baseline also fails the required ref0 qualification; do not suppress warnings or reduce the replacement profile.
