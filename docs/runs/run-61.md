tag: run-61
run: 61
date: 2026-10-13
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: main `22769f3`, soak: `enable = landvalue, weather, ordinance, demand, power, plumbing, flammability`, `popup_dialogs = 0`, `observe = 1`, `autotest_end = quit`, 900 s at speed 2 (2026 → 2074, about 48 game years)
result: **clean, `ref_warnings=0`, 0 foreign exceptions, no stalls, quit path clean.** R population 15,025 → 28,283, funds 834 → 409,959. Two findings: power and plumbing stayed `enabled=0` all game (ConnectionsReady never arrived; runs 62–66), and demand stayed frozen (the demand glue cannot read the BuildingProfile capacity lists or fill `GetLocalPopulationSummary`, which need the STLport map layout; RE ABI lane).
