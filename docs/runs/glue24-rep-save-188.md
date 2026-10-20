tag: glue24-rep-save-188
run: 205
date: 2026-10-20
lane: glue24
dll: FreeCity4-glue24
pacing: turbo
verdict: corrupt
switches: `enable = netmgr,netlot` (both replacements) with the save script (mark/digest/save/mark/digest), turbo to 2027-01-01 on Konradshohe, queue request `glue24-rep-save`
result: **corrupt (foreign_exceptions=1, ref_warnings=0), 30 s wall, played to 2027-01-02.** The script ran complete: `mark glue24-save-before`, both digests (before 2026-03-01 funds=1037 population R/C/I=15015/2469/7634; after 2026-03-02 funds=1037 population R/C/I=15015/2491/7828), and `script save -> ok=1 (SaveCity(full) in 10828 ms)`. `netmgr Write -> 1 (6 bytes, record53 round trip same)`; `netlot Write -> 1 (3160 bytes, record11 round trip DIFFERENT)` - the same delta the shadow save run recorded (one clock-seeded configuration draw, counted, not a mismatch). The crash: `first-chance exception OUTSIDE FreeCity4.dll in SimCity 4.exe+0x50D03F: ACCESS_VIOLATION (writing 0x18CC4004)` at the lot manager's `Release` under `~cFC4NetworkLotConfiguration -> SC4Vector<cSC4LotConfigurationObject>::deallocate` - the same double-owner defect as run 185 (SC4Vector's push_back memmoves the element bytes, so an AddObject by value shared the temp's `data` buffer). Fixed by building the records in place (AddObject takes the fields and copies the data values one by one).
summary: the save itself passed; teardown hit the memmove data owner again
