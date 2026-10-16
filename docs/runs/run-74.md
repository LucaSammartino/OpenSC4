tag: run-74
run: 74
date: 2026-10-14
lane: run
dll: ccdc3051
pacing: unknown
verdict: corrupt
switches: main `4619a6f` (DLL `ccdc3051`), soak 3 on Konradshohe: **all ten replacements** (`enable = landvalue, weather, ordinance, demand, power, plumbing, flammability, pollution, protection, crime`), `popup_dialogs = 0`, `observe = 1`, quit, 900 s
result: **`corrupt` 15 s after the load (2026-09-26, 8 months).** One first-chance access violation in the game: `SimCity 4.exe+0x1B9FEB`, writing address 0. It is the game's pooled allocator (`0x5B9FC0`, used by the global `operator new` `0x5E55E0`): a block over 4 KiB goes to `malloc`, which returned NULL, and the header write faults. No FreeCity4 frame in the top 1 KiB of the stack; the last calls were the budget getters of the month's UI refresh. Run 76 shows memory was not short, so the game asked for a garbage size. Under investigation (run 76 and later).
