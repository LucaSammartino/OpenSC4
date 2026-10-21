tag: glue29-nettool-221
run: 221
date: 2026-10-21
lane: glue29
dll: FreeCity4-glue29
pacing: turbo
verdict: clean
switches: `enable = simulator,netmgr,nettool` (the simulator, the replacement network manager and the network tools), turbo to 2032-01-01 on Konradshohe, 20 scripted `network ax az bx bz type` actions (FC4HarnessLogic.h; roads type 0, streets type 3, power lines type 5, avenues type 6; straight and L runs at 12-69 cells), queue request `glue29-run221`
result: **clean**, `ref_warnings=0`, rtc=0, overruns=0, foreign_exceptions=0, structured exceptions=0, other WARNING lines=0 (the 2 expected x87 start-up lines only). Every scripted placement `ok=1`: 8 roads/streets of 8 cells, L-runs of 21/24/29 cells, a 69-cell road. The `nettool type T PlaceNetworkOccupants` lines report `inserts=N refused=0 joins=0 geom[dragged=N valid=N solved=N failure=0x00000000] core[city=128x128 cache=0 refused=0]` — the fc4_nettool core drives the drag, the passes solve and the occupant inserts through the netmgr replacement; the power-line (type 5) and avenue (type 6) placements whose validation refuses (0xF0000006 / 0xF0000001) place nothing and return cleanly. 69 cumulative inserts on the type-0 tool, 16 on type 3, `nettool city shutdown: 4 live tool(s)`, and the full Shutdown path (`nettool type T Shutdown`, `netmgr Shutdown`) ends the run. refaudit: no over-release, no NEGATIVE in the required (budget Shutdown) summary; the simulator-shutdown summary shows the run-held occupants released by the city teardown.
summary: the network-tool replacement places roads and streets in the real game (inserts=8..69, valid=solved, failure 0), 20/20 scripted placements ok, city runs to 2032 clean with ref_warnings=0.
