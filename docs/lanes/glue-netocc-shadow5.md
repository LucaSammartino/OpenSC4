| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-shadow5: network occupant record shadow | Codex / glue-netocc-shadow5 | wip | Resolve NO45, then implement the record shadow and validate through 2032. |

## Changes

Commit 2b25736f corrects network-occupants.md §2.3 field 12 in the core segment-record codec.
The cached path-info value is a three-u32 resource key, including twelve zero bytes when absent;
the old implementation encoded an object reference (four bytes for null). `NetworkBaseRecord`
now retains a `ResKey` as `path_info_key`. Live resource resolution remains the host's responsibility.
Regression tests check exact key/i64 placement, absent and nonzero keys, old versions, and truncated tails.

## Validation

- Focused record tests: 15 passed, 0 failed. Explicit-file fmt: 2 files, no diff.
- Final stable-source `qbuild.py all`: PASS. Workspace cargo: 6231 passed, 0 failed, 0 ignored;
  clippy: exit 0, diagnostics 0; native build: PASS 217/217 edges; full CTest: 68/68 passed.
- Native request `qbuild-20261023-195433-build-ctest` was dispatched by the all gate through the
  coordinator queue. The original operation was collected to terminal PASS; no second verify was launched.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- Earlier invocation failures are separate: Git Bash wrapper CreateFileMapping failure
  (`build-runs/q/20261023-194939-8776.log`), then relative-path fmt file lookup errors.
  Direct PowerShell cargo and absolute-path fmt succeeded; no failed native gate was rerun.

## Question and remaining work

NO45 was appended to `docs/specs/questions/network-occupants.md`. Specified native Read clears
network flag bits `0xE000`, forces triangle alpha, and can canonicalize transform masks. Thus literal
Read then Write is not an unconditional inverse of native Write. The new focused test demonstrates
`0x06008000` becoming `0x06000000`. A byte-exact shadow needs a specified lossless-decoder or
normalization boundary; changing faithful live Read behavior to hide those differences is inappropriate.
The coordinator should regenerate the question index using its authorized tooling.

The requested plugin shadow remains unimplemented pending NO45: no ini key, C ABI adapter,
plugin hooks, regression profile changes, or game requests were added. C ABI exports are unchanged.
There is no game run record or `<label> <clean>/<total>` coverage result, and no verified coverage claim.

Next: settle NO45, implement the default-off load/month record shadow using the existing capture helpers,
then run the full gate and a real-city run through 2032 with every shadow line exact and ref_warnings=0.
