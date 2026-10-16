tag: m850-traffic-replace-105
run: 105
date: unknown
lane: m850-traffic-replace
dll: unknown
pacing: turbo
verdict: clean
switches: the same DLL, queue request `m850-traffic-replace` (priority high, slot 3, at the same time as run 104): **18 replacements, the regress list + `development` + `traffic`**, turbo to 2032-01-01
result: **clean, `ref_warnings=1`**, 27.0 days/s, 98 s. **First full-city run of our traffic simulator**: 72 traffic month lines, 10,054 routes in the last month; the city ends close to run 104's replace run (funds 91,716; R/C/I 22822/2615/10922). The one warning is the traffic glue's: `released last reference to game object … class 0xC9BD5D4A … ReleaseToken:599`, 30 s in; traffic-only runs never showed it. Back with the traffic lane (phase 2).
