# disasters: open questions
<!-- Questions about [disasters.md](../disasters.md). Append new entries at the END; when one is answered, move it to answered/disasters.md. Rules: README.md. -->

## From the disaster layer glue (filed by the coordinator from the `glue13` lane's report, 2026-10-16; merged `e7386b0`)

glue13 wired `fc4_disasters_host` to the game (`FC4DisasterHost`) and added `disasters_shadow = 1` (`FC4DisasterShadow`):
the original layer runs and a mirror core follows it (ABI 67, taken as 61 on the lane). The lane's final run
dis-final-11 was clean: fire 211/263 lines exact, riot 2/354, quiet 0/1, and every other difference is tagged with one
of the questions below in the log. The lane's Q36 (the riot's AttemptDisasterStart) is answered by disasters.md §6.2
(it always returns null; the shadow takes the instance from the first riot RegisterDisasterLocation). The lane's own
numbers (its Q26-Q42) are given in parentheses; the current choice is in brackets.

<!-- from: From the disaster layer glue (2026-10-16) -->
- **DS28:** (Q29) toxic spill, terrain, pipe burst and volcano are not mirrored: no start path was exercised. [fire and
  riot only]
