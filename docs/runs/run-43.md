tag: run-43
run: 43
date: 2026-10-13
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: the refaudit fix `A27E5D41…` (merged as `a07aa6a`), same config
result: **clean, `ref_warnings=0`**, and 160 of 160 NewDay updates bit-exact again. **One finding:** on 2001-10-01, 11 of 12 *tax modifiers* differ by **4 ulp** (orig `0.0832541287`, ours `0.0832540989`; I-R matches), on the day the census supply changed. The other 5 modifier checks are exact, and run 42 had that same day exact. Under investigation by the engine lane: a different input or sampling moment is suspected, not the formula. **Resolved by run 44.**
