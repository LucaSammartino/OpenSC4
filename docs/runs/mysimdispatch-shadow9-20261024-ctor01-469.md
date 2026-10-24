tag: mysimdispatch-shadow9-20261024-ctor01-469
run: 469
date: 2026-10-24
lane: glue-mysimdispatch-shadow9
dll: FreeCity4-glue-mysimdispatch-shadow9 (source 1f51abb5c, sha256 prefix c05fd09d)
pacing: turbo
verdict: clean
switches: train187 house shadows plus mysimdispatch_shadow=1; Konradshohe; end_date=2032-01-01; seconds=300; timeout=480
result: INCOMPLETE / audit failure: harness verdict clean, foreign_exceptions=0, ref_warnings=73, RTC=0, stalls=0; stopped by seconds limit at 2028-04-19 (301.734 s), not 2032. Exact line: mysimdispatch 27/27. Each sample constructor=1, live_headers=0, fields=7, errors=0, mismatches=0 (189 fields). No load sample: PublishedJulian unavailable at load callback. Warnings identify the fixture factory reference not registered with FC4_ACQUIRED before FC4_RELEASE_CREATED; corrected source adds that audit registration and reads the load date from the public simulator. Constructor/header scope only; no live trip verification or whole-owner credit. Preserve this run separately from corrected-source runs.
