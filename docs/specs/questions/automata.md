# automata: open questions
<!-- Questions about [automata.md](../automata.md). Append new entries at the END; when one is answered, move it to answered/automata.md. Rules: README.md. -->

## From the automata state behaviours and the effect-directory measurements (re-integrate1 lane, 2026-10-16)

Four RE reports, kept verbatim in `build-runs/re-integrate1-automata.md`, `re-integrate1-effects.md`,
`re-integrate1-vehicle-ticks.md` and `re-integrate1-air-water-ticks.md`. Their findings went into automata.md
§6.2, §6.5, §6.6, §8.1, §12-§14 and effects.md §3, §5.2, §7, §10.1, §10.2, §15-§17.

**Nothing needed striking here for the automata state questions**: automata.md's Q3-Q5 (filed above as AT3-AT5)
and AM1-AM12 were already answered by re-answers16 and re-answers20, and the effects questions EF1-EF5 lived in
effects.md §16 rather than in this file. Two of those are now closed or answered and are struck below so a
reader does not re-open them. AT6-AT19 continue automata.md §13's new Q7-Q20; EF1-EF10 continue effects.md §16.
The probe named is the one the report gives. **All of AT6-AT19 were answered on 2026-10-20 by re-answers33**
and moved to `answered/automata.md`.

## From the glue19 lane (2026-10-20)

**AT20.** Automata.md section 9 gives the U-Drive-It pause and resume hooks only from the manager's side
(slots 9 and 10) and says the city situation manager drives them, but the spec does not name the manager's
side: what city-situation slots or messages reach `fc4_automata_pause_all` / `_resume_all`, and when exactly
(countdown start, countdown end, mission end)? A replacement wiring this from the outside (glue19 wires them
to the manager's `MissionActivate` / countdown-end points through the city-situation interfaces) needs the
receiver names to check it against the game's own call sites.
