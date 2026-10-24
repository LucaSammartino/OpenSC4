| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-rzstrings-shadow5: strings shadow | Codex, glue-rzstrings-shadow5 | ready: cargo 6759, clippy clean, CTest 69/69; strings 73/73 through 2032 | Land measured subset; credit coverage and add regress key; resolve RZS8–RZS11. |

## Report

Added `strings_shadow=1`, registered as a known ini key and off by default. The
monthly observer constructs private original strings from a live city-name prefix
(96-byte bound) and four witnesses: ASCII search, ASCII case equality, counted NUL,
and empty. It compares native operations with the production `RzString` through a
stateless private transport. Every operation records hex inputs, native/core bytes,
raw scalar results and refusal status. Only private strings are mutated; no locale,
replacement dispatch or borrowed city state changes. No public C ABI/DLL exports
changed and no ABI version bump.

Measured operations: counted FromChar, Copy, Resize, Append, Insert, Replace, Erase,
sensitive/ASCII-insensitive IsEqual and Find/RFind, specified equal-prefix CompareTo
length differences, and Sprintf `%s`, positive `%+08d`, `%08x`, `%%`. At most 78
comparisons/month; the actual run had 78 comparisons and two explicit exclusions
(empty-haystack Find) in each of 73 months.

Final gate: `qbuild.py --wt <WT> all` PASS on stable source 0444fd703. Workspace
tests **6759 passed, 0 failed, 0 ignored**; clippy **0 diagnostics**; queued native
build **52/52 edges**; full CTest **69/69**. Native request
`qbuild-20261024-011602-build-ctest` completed PASS. Logs are
`build-runs/qbuild-{test,clippy}.log` and `build-runs/fc4verify-{build,ctest}.log`.
Owned Rust fmt passed. Two focused transport tests passed; focused return-channel
check reported zero unreviewed uses (73 reviewed).

Final measured tally: **`strings 73/73`**. Run
[strings5-2032-20261024-b1](../runs/strings5-2032-20261024-b1.md) was clean through
**2032-02-01**, DLL SHA256 prefix **b4f4e59d**, with **ref_warnings=0**, zero foreign
exceptions/stalls/panics/structured exceptions/incomplete shadows/other warnings.
All **5694** operation records were exact; every dated shadow line had mismatches=0.
First line 2026-02-01, last line 2032-02-01. Full log/verdict remain in
`build-runs/game-results/strings5-2032-20261024-b1.*`. No outstanding operations.

Initial evidence is preserved. Native build passed, but CTest was 68/69 because the
return checker classified unresolved `auto& s.IsEqual` as a date-interface call.
An explicit cIGZString receiver and spec-backed Boolean citation fixed the checker.
The original failed full log is `build-runs/evidence-run4/strings5-initial-ctest-fail.log`.
[Initial game a1](../runs/strings5-2032-20261024-a1.md) was clean through 2032-02-17,
ref_warnings=0, but `strings 0/73`: nine differences every month. Those native
observations became RZS9–RZS11; core behavior was not changed from observations.
The disputed paths remain excluded, rather than normalized or credited as exact.

Open questions in `docs/specs/questions/rz-strings.md`: **RZS8** native substring/case
helper ABI; **RZS9** unequal-prefix CompareTo magnitude; **RZS10** alternate hex plus
zero-padding order; **RZS11** empty-haystack Find exception. RZS7 finite decimal
extraction was absent at starting HEAD and remains explicitly excluded. Other
unmeasured scope: non-ASCII transforms/formatting (existing RZS2/RZS5), pointer
formatting, natural traffic interception, nonvirtual helpers, detokenizer and the
language utility/manager/language owners. This is constructed native measurement,
not a claim of natural-call interception or exhaustive owner coverage.

Coordinator follow-up: regenerate the questions index, credit the measured shadow,
and add `strings_shadow=1` to the regress shadows profile now that b1 is exact.
The engine lane did not read/edit `tools/regress.ps1` or run coverage tooling because
the clean-room rules prohibit the entire tools directory. No coverage mapping was
edited. Publish the detached HEAD commits to the reserved lane branch as usual.
