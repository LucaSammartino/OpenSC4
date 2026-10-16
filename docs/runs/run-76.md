tag: run-76
run: 76
date: 2026-10-14
lane: run
dll: bbcc67ea
pacing: unknown
verdict: clean
switches: main `f518e37` (the new report: registers, executable frames; heartbeat memory; DLL `bbcc67ea`), run 74 repeated: all ten replacements on Konradshohe, 300 s
result: **clean** (2026 → 2041), 0 problems; it passed 2026-09-26 without a fault. Memory flat: address space 715–722 of 2047 MiB used, largest free block 955 MiB, private 273 MiB. **So run 74's NULL from `malloc` was a garbage size (over 955 MiB), not a shortage.**
