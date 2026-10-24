tag: nettool-shadow5-r2-20261024-d-368
run: 368
date: 2026-10-24
lane: glue-nettool-shadow5
tip: d60e1d60a
dll: a980ec0b
city: Konradshohe
duration: 40.468 s, through 2032-01-15
pacing: turbo
verdict: clean
switches: enable=simulator; disable=nettool,netmgr; nettool_shadow=1; script=engine/tests/nettool-shadow.script
result: nettool-shadow5-r2-20261024-d 8/14; ref_warnings=0; foreign_exceptions=0; 53.86 days/s. Final stable source 65082c135, full gate cargo 6760/0, clippy clean, native build 50/50, CTest 69/69. All fourteen shadow lines identical to normal run b and earlier turbo runs a/c. All ordered footprints exact; remaining four costs are NT30 and powerline/bridge replay differences are NT31. Exact line: nettool shadow (type=6 drag=(4,28)-(16,28) scope=drag/status/cost): mismatches=0 cells=26/26 cell_diffs=0 ok=0/0 failure=00000001/00000001 valid=0/0 straight=13/13 total=13/13 cost=0/0 uncovered=pieces,bridges,poles,wires contract=NT27-NT29
