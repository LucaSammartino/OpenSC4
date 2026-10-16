tag: run-99
run: 99
date: unknown
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: the r8a DLL (`d8d305bf`), `enable = zones` only, Konradshohe, two zone drags with the new `tools/sc4ctl.ps1 realdrag` (Mayor mode → zone → residential → low density at client (43, 70), (100, 45), (176, 40); the 3D view needs real input), no save
result: **clean** (`AUTOTEST VERDICT: clean`, `ref_warnings=0`, to 2047-04). **First in-game run of our `PlaceZone` commit path and the dezone handler.** While dragging, the UI prices with dry runs (`type=1 rect=58,91..64,93 cost=180 cleared=3`, growing with the rect). On release our zone manager commits twice: `type=0 … accepted=8 cost=0` (the street cells the tool lays), then `type=1 rect=58,91..64,93 cells=21 accepted=18 cost=180 cleared=3`. The new streets' InsertOccupant messages then dezone their cells through our handler (90 lines, `changed=1 lots=0`). The month line goes from 0 to 10 residential-low cells (18 accepted, 8 under streets); a second drag over a larger rect (57,90..65,93, 20 accepted, 16 cleared, cost 100) ends at 18. No warning follows (the original zone developer's parcelling is not logged). Every replaced subsystem logs Shutdown twice (residential and power too): the game calls it twice. Posted clicks on the fourth flyout icon did not select the dezone tool; the dezone-tool path is still untested.
