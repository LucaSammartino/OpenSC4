| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-scres-shadow5: resource shadow | Codex, `glue-scres-shadow5` (reserved branch, detached HEAD) | WIP: final gate passes; both real-city runs clean to 2032 but `scres 0/73` each (empty natural sample). No verification credit. SCR8/SCR9 open. | Publish native ABI and natural capture routes, complete capture, then repeat the real-city measurement. |

## Implementation and scope

Added default-off ini key `scres_shadow` and original-forwarding hooks for the
specified binary/text token adapter numeric, descriptor and resource-key read
slots. Installation starts at director OnStart before framework init, checks
entries against specified originals, and restores them at shutdown. The original
executes once and retains its result. Borrowed pointers are not retained/released.

Private static-link transport replays consumed bytes through production core
`BinaryIo`/`TextIo`, comparing output bytes, boolean result, counter and consumed
length. Capture uses SDK raw read/seek operations, restoring the original
post-call position. SCR8 requests a passive byte accessor. No public C ABI exports
changed and no ABI version bump.

Dated load/month lines say `partial=adapter_reads`; an empty sample reports
mismatches=1. String, command/file parser, image/buffer and ASCII/text writer
capture remains off pending complete native argument/access contracts. This
does **not** capture every resource file or qualify the subsystem as verified.
No coverage map or regression profile was edited.

## Validation

- Final source `a8f493cbe`; subsequent commits contain reports only.
- Focused tests 3/3: byte/status/counter differences, text narrowing/descriptor,
  strict binary cutoff. Absolute-path fmt check passed.
- Final `qbuild all` **PASS**: workspace cargo **6806 passed**, 0 failed/ignored;
  clippy `-D warnings` 0 diagnostics; native build **49/49** and full CTest
  **70/70**. Native queue `qbuild-20261024-013221-build-ctest` terminal PASS,
  112.2 s. The all command itself queues native verification; none duplicated.
- Logs: `build-runs/qbuild-{test,clippy}.log`, `fc4verify-{build,ctest}.log`.
- Initial source `16f94847c` also passed (6806 cargo, clean clippy, build
  224/224, CTest 70/70). First game exposed unknown-key/load-date diagnostics.
  A correction failed native compilation because CurrentCity is observer-local;
  CTest skipped. Evidence preserved in
  `build-runs/fc4verify-build-failed-77c1e6c05.log`. Final source passes the
  borrowed city from the observer instead.

## Real-city results

- [scres5-r1-2032-a, run 349](../runs/scres5-r1-2032-a-349.md): clean to
  **2032-01-19**, ref_warnings=0; exact summary **`scres 0/73`**. Installed
  before PreAppInit; calls=0, bytes=0, refused=0. Ini/date diagnostics later fixed.
- [scres5-r1-2032-b, run 350](../runs/scres5-r1-2032-b-350.md): clean to
  **2032-01-05**, ref_warnings=0, all problem counts and other warnings zero;
  exact summary **`scres 0/73`**. Installed at OnStart; still calls=0, bytes=0,
  refused=0. Correct load date 2026-01-27; load plus 72 monthly lines.

Both used Konradshohe, sim_turbo=1, scres_shadow=1, end_date=2032-01-01.
Final DLL SHA256 prefix c788d59f. Sequential unique requests, each collected by
one event-driven blocking wait. No outstanding game or native build requests.
No native value differences were observed because no comparisons occurred.
Mismatch counts report missing observations, not a measured turbo-only difference.
A normal-speed repetition cannot establish the missing native ABI/capture route.

## Open contracts and next steps

Appended **SCR8** (complete native signatures and passive output access) and
**SCR9** (natural/direct capture dispatch, both zero-sample observations) in
`docs/specs/questions/sc-resources.md`. The coordinator must regenerate the
question index; clean-room lanes cannot run tools/ scripts.

Obtain those contracts, capture the game's actual parser/token/image/writer
traffic, then gate and measure through 2032 again. Nothing is presently measured
by this subset, so enqueue **WIP**, as required for an entirely empty natural
sample. Keep the key default-off and out of the regression shadow profile.
**Do not credit verified coverage from this lane.**
