| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-scres-shadow5: resource shadow | Codex, `glue-scres-shadow5` (reserved branch, detached HEAD) | WIP: constructed adapters exact 56/56, clean 2032 city, full gate PASS; broader parser coverage unresolved | Publish SCR8/SCR9 contracts, complete whole-resource parser capture, add the regression profile key through a permitted tooling lane. |

## Round 2 implementation

Implementation commits: `985f7d3a4`, corrected by `ac743eddc`. Added default-off
`scres_constructed`, independent of passive `scres_shadow`. At director startup
it directly calls the specified native binary/text adapter read entry points on
fixed section-6 inputs (including SCR-V13–15/V26 and boundary/overflow cases).
Calls are gated by executable identity and original table-entry checks. Unknown
methods refuse; no unknown constructor or destructor is invoked.

At city load it opens the fixed shipped pack-1 INI through the game's resource
manager, reads its first 16 bytes, releases only its acquired serial reference,
and closes the manager-owned record. Those bytes feed the native binary numeric
adapters and the production Rust port independently. This measures primitive
reads from a real resource prefix; it does **not** measure the INI parser or a
complete resource-file parser. The SDK-shaped bounded read-only input record
counts unsupported operations as refusals. Borrowed adapter pointers are never
released. The private observation transport now compares full input and consumed
cursor separately, allowing leftover bytes and strict-boundary failures to be
checked. No public C ABI exports changed.

## Round 2 validation

- Focused Rust tests **4/4**, owned-file fmt PASS.
- Final corrected-source `qbuild all` **PASS**: workspace cargo **6807 passed**,
  zero failed/ignored; clippy `-D warnings` zero diagnostics; native build
  **91/91** (6.3 s), CTest **70/70** (96.5 s), native total 104.6 s.
  Request `qbuild-20261024-015022-build-ctest` collected to terminal. Only lane
  reporting changed while it ran; implementation source remained stable.
- Initial native attempt `qbuild-20261024-014455-build-ctest` failed at 47/49
  because the anonymous-namespace helper omitted `fc4::` on `compat`.
  Corrected in `ac743eddc`; full failed evidence retained at
  `build-runs/fc4verify-build-failed-985f7d3a4-r2.log`.
- Initial focused q.py invocation failed before cargo (Bash CreateFileMapping
  error 5); direct cargo test then passed. Its failed wrapper log remains
  `build-runs/q/20261024-014247-10060.log`.
- Final logs: `build-runs/qbuild-{test,clippy}.log`,
  `build-runs/fc4verify-{build,ctest}.log`.

## Round 2 real-city measurement

[scres5-r2-constructed-birch](../runs/scres5-r2-constructed-birch.md), Konradshohe,
turbo through **2032-01-07**, **clean**, `ref_warnings=0`. All other problem
counts and unexpected warnings zero. One unique request, one event-driven
verdict wait, no qwait, no outstanding operation. DLL SHA256
`90e90cf88f45cbd1196cc87abfc65b4b3f9d2c3ba95de4690cc11f9d1d6c8dc0`.

| Input group | Exact/total | Refused | Mismatches |
|---|---|---|---|
| Fixed binary adapter vectors | 24/24 | 0 | 0 |
| Fixed text adapter vectors | 22/22 | 0 | 0 |
| Shipped pack-1 INI raw prefix, numeric binary reads | 10/10 | 0 | 0 |

Total **56/56**, harness summary **scres 3/3** dated lines exact. All lines
explicitly say `partial=adapter_reads`. Real resource key
`00000000/8A5971C5/2B563701`, byte window `0:16`, FNV-1a `6AE1A633`.
No turbo-only mismatch occurred, so a normal-speed rerun was unnecessary.

## Remaining work and clean-room boundary

The task's request to inspect ghidra/decompilation and run tools scripts
conflicts with the explicit non-bending clean-room rules. No forbidden source
was opened or listed, no tools script was run, and no spec behavior was invented
from ABI word counts. **SCR8/SCR9 remain open** in
`docs/specs/questions/sc-resources.md`, with round-2 implementation and measured
evidence appended. No question was marked answered; no behavior spec, answered
archive or generated index was edited.

The command/file parser, strings, token-owner construction, image/buffer and
ASCII writer signatures/accessors still require publication by a spec/RE lane.
Natural whole-resource dispatch remains unresolved. `tools/regress.ps1` is
explicitly prohibited to this lane and remains unmodified; a permitted tooling
lane must add `scres_constructed` to its single existing `shadows =` string.
Complete those contracts and full-resource capture, then repeat the game
measurement before crediting the subsystem or setting the merge entry ready.
The merge entry is **WIP**, despite the measured adapter checks being exact.
HEAD remains intentionally detached; no branch/ref was switched or published.

## Round 1 evidence (retained)

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
