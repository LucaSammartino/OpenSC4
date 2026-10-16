tag: run-48
run: 48
date: unknown
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: same, 300 s, watched on screen
result: **clean.** At 2026-05-03 an advisor popup ("Educators Take Long Recess") held the city, then a second queued behind it. Each popup calls `HiddenPause()` on our simulator (`hidden` 1, then 2). Esc and Enter did not close them; clicking Close did, and the date moved on. The `hidden` heartbeat field *is* the popup signal (HANDOFF §8 said otherwise).
