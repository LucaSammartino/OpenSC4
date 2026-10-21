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

<!-- from: verify-disasters4 (2026-10-21) -->
- **DS41:** §5.4 step 4 says an occupant rectangle with no overlap with the city "gives an empty rectangle", then
  averages it with `GetAverageValueInCellRect(x, z, w, h)` (`sum / (w × h)`). With w × h = 0 the division (and
  simgrid §4.1 step 1's one-tract test with `x + w − 1 < x`) is not specified: what coverage byte does the original's
  step 4 (and §5.6's spread) read for such an occupant, and does it refuse? [the plugin reads 0 coverage, so it does
  not refuse at step 4; unreachable for an occupant inside the city]

<!-- from: glue-disasters4 (2026-10-21) -->
- **DS42:** §1 says the city creates the layer in `CreateCityComponents` (`0x0045B01C`) and gives its class
  `0x61F6ABF5`, but not how. Is it created through GZCOM (`GetClassObject` / `CreateInstance(0x61F6ABF5, iid)`), so
  that a class factory a plugin registers for `0x61F6ABF5` replaces it (as flammability.md §1 states for its class),
  or by a direct constructor? If through GZCOM: which IID is requested, how many references the city holds at
  `+0x1C4`, and what the city calls at teardown (`Shutdown` slot 4, `Release`), in which order relative to the other
  components (the managers' `Shutdown` releases instances that post messages and touch other subsystems).
  [blocked: an `enable = disasters` layer replacement needs this boundary; nothing implemented]
- **DS43:** §2.3 names a cached layer pointer `0x00B43D3C`. Who writes it (the layer's constructor or `Init`, the
  city, a getter), when, and who clears it? Which code outside the layer's own functions reads it, and which layer
  slots does it call through it? Likewise the layer pointers kept by the effect and toxic instances: taken from city
  slot 72 or handed over by the layer, and AddRef'd or not? A replacement layer cannot know whether native readers of
  the global would see null or a stale pointer. [blocked with DS42]
- **DS44:** Is the `cISC4DisasterLayer` interface (§15) the whole boundary to the native managers, instances and
  other consumers? That is: does any code outside the layer `QueryInterface` the layer for `0x452294AA` (its message
  target at `+0xC`) or another IID, or cast the interface to the concrete class and read fields directly (other than
  through the `GetDisasterCost` record pointer of §2.6)? [blocked with DS42]
