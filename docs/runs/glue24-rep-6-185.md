tag: glue24-rep-6-185
run: 185
date: 2026-10-20
lane: glue24
dll: FreeCity4-glue24
pacing: turbo
verdict: corrupt
switches: `enable = netmgr,netlot` (both replacements), turbo to 2032-01-01 on Konradshohe, queue request `glue24-rep-6`
result: **corrupt (foreign_exceptions=1, ref_warnings=0), 46 s wall, played clean to 2032-01-06 (julian 2463238, 32.7 s, 66.45 days/s) and crashed at teardown.** `netmgr Init: ok=1 ... tools=13` and `netlot Init: ok=1 lots=525 potholes=0 configs=543` - the tools and the 543 configurations load. The crash: `first-chance exception OUTSIDE FreeCity4.dll in ntdll.dll+0x4E728: ACCESS_VIOLATION (reading 0x30000B72)` under `~cFC4NetworkLotConfiguration -> std::vector<cSC4LotConfigurationObject>::_Tidy -> cSC4LotConfigurationObject deleting destructor -> SC4Vector<unsigned int>::deallocate`, reached from `cFC4NetworkLotManager::~cFC4NetworkLotManager` clearing `configs_`. Diagnosis: the configuration held its object records twice (a `std::vector` build storage and an `SC4Vector` for slot 5 memmove-built from it), so each record's `data` SC4Vector was destroyed twice. Fixed by the single-storage change (one `SC4Vector<cSC4LotConfigurationObject>` only).
summary: rep-6 played to 2032 and died at teardown; the configuration records were owned twice
