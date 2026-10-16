tag: run-54
run: 54
date: unknown
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: same, interactive: God mode → Disasters → fire on a building
result: **Emergency pause verified:** `Pause()` (pause 1) → `EmergencyPause()` (pause 2, emergency 1, red border and warning icon) → `EmergencyResume()` ~11 s later (pause 1, emergency 0) → `Resume()` (0), and the date moved on. No fire took hold, so this checks the pause calls, not a burning city. The 3D view ignores posted clicks: choosing the fire and clicking the city needed real input (`sc4ctl realclick`).
