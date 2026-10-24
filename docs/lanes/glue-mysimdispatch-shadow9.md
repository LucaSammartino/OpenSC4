| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-mysimdispatch-shadow9: scoped dispatch constructor shadow | `Codex` / `glue-mysimdispatch-shadow9` (engine lane, detached HEAD; supervisor publishes) | ready for measured scope only; cargo 6986 passed, clippy clean, CTest 79/79; run 470 clean to 2032, mysimdispatch 73/73 exact, ref_warnings=0; MS20 filed, MS18 open; live owner unverified | 1. Answer MS20 and regenerate question index. 2. Extend to live independent replay. 3. Close MS18 before whole-owner credit. |

## Measured result and scope

`mysimdispatch 73/73` — exact from 2026-01-27 through 2032-01-01 in the real city, using the train187 house ini plus `mysimdispatch_shadow=1`. Run [470](../runs/mysimdispatch-shadow9-20261024-ctor02-470.md), label `mysimdispatch-shadow9-20261024-ctor02`, reached 2032 cleanly: foreign_exceptions=0, ref_warnings=0, stalls=0, RTC=0, real_seconds=820.547, wall=839 s. PreAppShutdown completed. No outstanding requests.

**This is constructor-fixture evidence, not whole-owner verification. Do not credit the cSC4MySimDispatch* coverage row whole from it.** Every sample had constructor=1, live_headers=0, fields=7, errors=0, mismatches=0: 511 independent field comparisons. The city had no discoverable live dispatch records. No queue or transport behavior was exercised by these fixtures.

## Implementation

Default-off ini key `mysimdispatch_shadow` (environment `FC4_MYSIMDISPATCH_SHADOW`) is registered in FC4Config. FC4Observer invokes the new shadow at city load and monthly, deduplicated by calendar month, and clears its counters on shutdown. The dated line uses the single lowercase label `mysimdispatch` and ends with the total mismatch count.

The adapter creates an original dispatch through the public COM class factory, captures its specified cIGZSerializable::Write payload and GetGZCLSID, then releases the created object. It never calls Init or Read on that fixture, never changes a live owner and never arms debug registers. FC4_ACQUIRED records the factory-owned reference; FC4_RELEASE_CREATED frees it. Temporary WriteCapture QI references remain paired by the existing capture utility.

The private static-link Rust comparator constructs a fresh production-core DispatchState::default independently on each comparison. Seven compared fields: class ID, writer version, constructor state, previous state, flags, minute clock and travel type. Native outputs never initialize expected state. Unspecified constructor tuning, opaque scalars and locations are deliberately not inferred. No public C ABI exports or ABI version changed.

Public advisor serialization is also scanned for references of dispatch class 0xCBC14674. When present, those live records compare class/version only; their evolving state is not loaded into expected values. None were present in this run.

## Owner methods and contracts still uncompared

MS20 blocks a safe live replay: the SDK has no dispatch interface header and the spec supplies no ordered callable dispatch slot/signature table. Owner notifications are synchronous calls, not message-server broadcasts, so subscribing to animation/minute messages cannot expose every request input. The question also asks how to discover inactive MySims’ dispatches and which other constructor defaults are guaranteed.

- Init/Shutdown and deferred setup; all four location getters, location equality/validity, SetDefaultDestination, SetCurrentCommuteDestination, job-search validation, FindNewJob and ChangeJob.
- §4.6.1 AddDestinationLocation options/front/back/clear behavior; clock-minute and animation updates; trip-state selection, deadlines, wait timestamp getter/reset; state/flag setters, counters and complete owner-notification order. Constructor values alone are measured, not these methods.
- §4.11 the five Lua tuning inputs and their runtime duration clamps, idle/switch deadlines, approach-cell test and proximity threshold uses.
- §7.2.1 LeaveForWork/ReturnHome/DispatchCommute, TeleportToCommuteLocation, GoTo including cancellation/stuck selection, CanUDriveIt, Enter/Exit and mission handoffs.
- §4.6.2 settled asymmetric rectangle geometry, float32 rounding/truncation, strict/inclusive proximity boundaries, ordinary-mode switch waits, first-match stop selection/purpose/group mapping, stop timestamps and notices, strict transit expiry and common notification tail.
- §6.2.1 Read and full live serialized payload: auxiliary/opaque fields, remaining scalars, all four locations/references, auxiliary string, automaton/owner/service identities, route deque, model map, route marker and destination queue. Only writer identity/version and the five settled constructor scalar defaults are measured here. No native-output roundtrip is presented as independent expected state.
- §8 absence of draws during every settled operation and delegated RNG behavior are not dynamically observed.

MS18 remains separately PARTIAL and uncompared: full branch/mask matrix (especially idle/FE/FF/cancellation), neighbour retry enumeration/index, EBA06BD0 and complete completion/recovery ordering, dispatch-tool input/pylon lifecycle, mission callbacks and delegated model/automata randomness. No unsupported transport branch is represented as a successful trip.

## Validation and retained evidence

Source milestones: cdaea5883 (scoped implementation), 1f51abb5c (month deduplication/format), 9e3c6df24 (factory audit registration and valid public load date). Focused Rust mutation tests pass 2/2: each of seven field differences is counted, live-header scope counts only its two fields, and null input is rejected. Owned Rust fmt passes.

Final stable-source `qbuild all`: cargo 6986 passed, 0 failed/ignored; clippy diagnostics=0; queued native `qbuild-20261024-085949-build-ctest` PASS, build 207/207 edges and full CTest 79/79. Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log and fc4verify-ctest.log. The same queued operation was collected to terminal PASS; no duplicate verify was launched.

Earlier source also passed the full gate (native 254/254, CTest 79/79), but [run 469](../runs/mysimdispatch-shadow9-20261024-ctor01-469.md) is unacceptable: 27/27 exact, stopped at 2028-04-19 under the initial 300-second limit, ref_warnings=73. Its factory acquisition lacked the auditor registration and its initial PublishedJulian was unset. Both were corrected in 9e3c6df24, then the full gate was repeated. The failed/incomplete evidence remains under its unique ctor01 label; corrected evidence uses ctor02. The shell wrapper could not start Git Bash; the initial focused cargo test ran directly instead.

## Questions and next steps

Appended **MS20** to docs/specs/questions/mysims.md. MS18 remains open. The coordinator must regenerate the questions index; this clean-room lane cannot run tools/ scripts. Coverage tables and the permanent regress profile are untouched: the new key remains opt-in, and this scoped fixture result does not justify whole-owner credit or promotion into the house profile.

After MS20 is answered, add typed native observation and complete input capture, feed those inputs to an independent persistent Rust dispatch and compare the remaining settled contracts field by field. Keep MS18’s remainder explicitly uncompared until its contracts close. Work was committed on detached HEAD as instructed; the supervisor publishes the reserved lane branch.
