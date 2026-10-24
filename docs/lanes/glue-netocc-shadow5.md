| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-shadow5: network occupant record shadow | Codex / glue-netocc-shadow5 | ready | Land; coordinator credits coverage and adds netocc_shadow to regression shadows profile. |

## Changes

Default-off `netocc_shadow = 1` captures the original occupant-manager object table,
then captures each supported network occupant's Write record in the same process.
The private static-link Rust transport reads and writes the record through the core
codecs and compares its bytes. No replacement objects are installed, no native
state is changed, and the public C ABI exports and version are unchanged.
The observer checks city load and every month; shutdown logs the clean/total tally.

The shadow covers network/prebuilt/bridge/tunnel records, underground/pipe/subway
records, underground/pipe/subway connections, power poles, power lines and pylons.
It reuses WriteCapture and its numeric reference identities. The opt-in variant
capture path reuses the existing traffic record helper, without changing default
capture behavior. Per-class counts and bounded diagnostics expose capture and
codec failures; an empty sample cannot count as exact.

Core fixes from earlier rounds: 2b25736f encodes the cached path-info resource key
as three words rather than an object reference; 7e82f20c corrects the NO13 v2
power-line position/cells/boolean/two-reference record. bbdb3554 fixes native
property-variant capture. Round 3 commit a58b9739 fixes the remaining pole input:
network-occupants §4.6 selects attachment tables from the exemplar resource key,
so the host now uses the decoded base holder's parent instance. The unrelated
model property 0x27812821 is no longer fetched or passed from the plugin.
The new regression covers all four valid attachment subtypes while the model
property has subtype zero, and retains the missing-key invariant.

NO45 was filed in an earlier round in docs/specs/questions/network-occupants.md
and resolved by the coordinator: compare after only the specified flag E000,
triangle-alpha FF and unit-scale transform-mask 7-to-5 canonicalizations.
Those transformations remain counted as fixups; other differences remain visible.
No new questions were needed in round 3.

## Validation

Stable engine source a58b9739 passed the single final `qbuild.py all` operation:
workspace cargo **6234 passed, 0 failed, 0 ignored**; clippy **exit 0,
0 diagnostics**; native build **103/103 edges**; full CTest **68/68 passed**.
Native build/CTest ran through the coordinator queue as
`qbuild-20261023-221501-build-ctest`, collected to terminal `qbuild: PASS`.
No duplicate verify request was started. Later commits change documentation only.
Focused capi shadow tests: **3 passed**; explicit-file fmt: **PASS**.
Logs: build-runs/qbuild-test.log, qbuild-clippy.log,
fc4verify-build.log and fc4verify-ctest.log.

Initial focused q.py invocation could not start Git Bash (CreateFileMapping
error 5); direct PowerShell cargo succeeded. The first fmt invocation used
the wrong engine-prefixed path; engine-relative capi/src/netocc_shadow.rs passed.
These invocation failures are distinct from the passing final full gate.

## Real-city evidence

**`netocc-r3-2032-1 72/72`**

Konradshohe, turbo=1, netocc_shadow=1, requested end date 2032-01-01;
terminal verdict **clean**, actual date **2032-01-05**, real_seconds=47.875,
DLL SHA256 prefix **428ffefd**, ref_warnings=0. All 72 dated monthly lines
(2026-02-01 through 2032-01-01) have mismatches=0. Including the unknown-date
load comparison, the plugin summary is **clean=73/73**. Every sample contains
1283 records / 478251 bytes: 857 network, 403 pipe, 2 pipe connections,
3 poles and 18 power lines. Monthly normalization fixups=857; load fixups=0.
No differences remain, so no normal-speed follow-up is required.
Classes absent from this fixture have zero sampled records; this run makes no
claim that those absent classes were exercised in the game.

Run record: [netocc-r3-2032-1-331.md](../runs/netocc-r3-2032-1-331.md).
Evidence: build-runs/game-results/netocc-r3-2032-1.log and .verdict;
build-runs/summarize-netocc.py prints the exact coverage line above.
All requests were waited to their terminal result; no builds or games remain outstanding.

Earlier nonexact runs are preserved separately:
[netocc-r2-2032-1-322.md](../runs/netocc-r2-2032-1-322.md), 0/73 samples before
variant capture was fixed; [netocc-r2-2032-2-323.md](../runs/netocc-r2-2032-2-323.md),
0/72 dated lines (0/73 including load), with only the three poles differing.
Their verdicts were clean, but neither run is claimed as exact coverage.

## Landing

Ready: full gate passed and every final-run shadow line is exact through 2032.
Public ABI unchanged. The shadow remains default-off. Coverage maps and tools/
were not edited under this lane's clean-room restrictions. The coordinator can
credit verified coverage and add `netocc_shadow = 1` to tools/regress.ps1's
shadows profile using the exact run above. No further engine changes are required.
