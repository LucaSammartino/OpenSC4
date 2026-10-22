| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dates4: DT2/DT3 adapter | `glue-dates4` / engine lane, detached reserved worktree | wip; ABI exports +3, version unchanged; cargo4940/0, clippy0, CTest68/68; native1314/1314 queries and185/185 fixtures exact; adapter/union40 clean2032/ref0; native save stalled; DT4/DT5 open | 1. Resolve published executable CRT host-input acquisition. 2. Investigate native save stall and prove reload of our newly written city. No full-owner promotion. |

## Handoff
- Goal: finish delivery of reconciled DT2/DT3 plugin/CAPI/core corrections; full-owner compatibility remains wip.
- Guard denied finalization at150k; no merge-queue file written. Stop and return continue after this commit.
- Done: code2ca78537/1e937686/f32af464; stable gate70c454c4 Rust4940/0, clippy0, CTest68/68.
- Done: adapter clean2032/ref0 recordedf413bd7b; union40 clean2032/ref0 andtwo saves recorded28ddbb95.
- Done after28ddbb95: native-nosave clean2032/ref0,1314/1314 query checks and185/185 fixtures exact; all failures/refusals/fallback/resync0.
- Next exact step: review the full report below, commit any final report adjustment, then write wip merge-queue JSON using actual HEAD and four recorded labels. No more builds/game runs needed for this delivery.
- DT4/DT5 remain open; preserve original native-save stall and unproven reload of our newly written city; no full-owner promotion.
- No requests outstanding. Frozen DLL and all logs remain under build-runs; SHA256 below.
- Detached HEAD intentionally; supervisor publishes commits. Do not change refs or restart work beyond authorized scope.

## Report

Core/CAPI milestone `2ca78537`; adapter/probe `1e937686`, final runtime code `f32af464`; final stable gate `70c454c4`. Existing simulator GetSimDate remains the insertion point; no new date factory, replacement key, default, map or ABI-table edits.

Rust now accepts explicit captured byte-folding input, bounds month parsing to three bytes, and tests embedded NUL/high bytes. Three additive C exports bridge scalar dates, month parsing and text. The adapter uses the published unsigned calendar, full EAX setters/Copy/out-Copy, Clone EAX1/ref1, and atomic date storage. DateString creates an actual native temporary, dispatches owning virtual Copy, and captures real native allocator cleanup EAX; it never synthesizes success. Unknown allocator builds refuse explicitly.

The opt-in dates_probe acquires a borrowed original through city.GetSimulator.GetSimDate and accepts only the published cGZDate interface vptr00ACEA38. It never releases that borrow. Mutating fixtures use owned native clones and audited releases. Query comparisons use identical captured day inputs and are stateless, not evidence of independently progressing replacement state. Fixtures cover zero/high unsigned days, source refs7, incoming EAX sentinel, clone independence after source release, byte/NUL/high-byte names, three readable bytes at a protected boundary, native/foreign owning outputs, foreign non-mutating Copy and EAX sentinels. Fault/allocation-failure cases and outer-lock depth are explicitly unexecuted. No cRZTime or unnamed helper ownership credit.

## Verification

Baseline all:4938 Rust tests, clippy0,68/68 CTests (baseline tree-change flag preserved). Focused quick:3453 tests and119/119 build edges. Final stable all:4940 tests, clippy0,68/68 CTests, no tree-change flag. Earlier full attempts67/68 exposed obsolete capped-trace assertions, corrected without runtime changes. Four date Rust files fmt PASS; lib.rs fmt traverses unrelated modules with existing formatting diffs, left untouched.

Frozen DLL: build-runs/dll/gd4-r1-final/FreeCity4.dll; SHA256 `4FCF41E234C93150A770B55CA013BDD13359B2528C5B8BBF65D78F3617279EC0`. All four immutable-label requests used it:

- `gd4-r1-native-a-20261022`: stalled at scripted save2027-01-02; foreign0/ref0,234/234 queries and185/185 fixtures, zero date mismatches/refusals/failures/fallback/resync. Not a clean2032 result; unresolved failure retained.
- `gd4-r1-adapter-a-20261022`: no-save control, clean2032-01-12/ref0,706710 ordinary adapter value calls;73 independent-reference refusals,0/0 comparisons, no equality credit. Original saved-city Read and full shutdown observed.
- `gd4-r1-union40-a-20261022`: supplied literal40 keys plus default simulator/budget; clean2032-01-01/ref0,195438 ordinary adapter value calls;73 refusals,0/0 comparisons, no equality credit. Full saves succeeded11094ms and11047ms, followed by full shutdown.
- `gd4-r1-native-nosave-a-20261022`: clean2032-01-01/ref0;1314/1314 observer-driven read-only queries and185/185 fixtures exact; refusals0/failures0/fallback0/resync0. Ordinary adapter calls0;1414 fixture value calls;9 unresolved-locale fixture calls; SetTime refusals0. This separate lifecycle control does not repair the native-save failure.

All clean runs foreign exceptions/watchdog/RTC/overruns0. Game verdicts/logs and wrapper logs remain under build-runs. The q.py Bash runtime failed E_ACCESSDENIED; bounded local Python waits ran directly through PowerShell, without qwait, executable inspection or game launching. No requests outstanding.

## Questions and limits

DT4 and DT5 appended to docs/specs/questions/dates.md: no published public acquisition of executable-CRT LC_CTYPE mapping or timezone/daylight inputs. Plugin CRT locale is not evidence. Month adapter remains explicitly C-locale-only; SetTime preserves its unimplemented path with counted refusal. No full-owner compatibility/replacement credit. The combined profile proves two saves and shutdown, but reload of our newly written game save remains unproven. No extra delivery round requested; this round preserves usable corrections and precise blockers.
