# Growth developer: open questions

- **GD-GC29-1 - state-18 inputs during unequal adaptive slices (sections 5.2, 14).**
  Census3 `gen27-konradshohe-s1-churn.shadows.txt` reports the 2027-12-03
  residential update with one population difference: tract (5,24), original
  85 versus shadow 226; final D, capacity and maximum capacity are exact.
  The spec defines per-tract W/capacity reads followed by
  `tractDeveloper.UpdatePopulation`. The shadow follows each original
  delivery, but its adaptive slice can consume different tracts per call,
  and its UpdatePopulation callback cannot write into the game.
  What original state/iterator/working-grid interface or documented offsets
  let a same-input shadow capture each tract's W and capacity at the original
  read? Can UpdatePopulation synchronously change a later tract's capacity
  or population before the original pass finishes? Please specify the input
  capture boundary; no population comparison or tolerance was changed.
  Corrected-precision reruns did not reach this historical date:
  `build-runs/suite/growth-cities29-5/gen27-konradshohe-s1-churn.verdict`
  is corrupt at 2027-11-02 (one access violation outside FreeCity4 at
  0x005D07CD), while the isolated development-only retry in tag 6 exits
  with code -1073741819 after a 2026-09-07 heartbeat. Their 263/263 and
  105/105 development lines are exact, but do not validate the late case.
  Blocks claiming this population case fixed; a clean late-date rerun is
  still required even if the precision correction removed the difference.
