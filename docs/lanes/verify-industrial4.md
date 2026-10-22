| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-industrial4: constructed native industrial comparison | `verify-industrial4` / `.claude/worktrees/verify-industrial4` (detached HEAD) | in progress, round 2/3 | Independent original/Rust comparison, gates and guarded game controls. |

## Handoff

- Goal: constructed ORIGINAL industrial versus independent Rust; IN3 natural interception excluded.
- Current base d88e9470, detached HEAD; prior round b8e853e6 filed IN3.
- Done: 1d8106c9 implements owned original versus direct Rust, validated typed views/image, all u16 versions and lifecycle/streams/messages/ref returns.
- Baseline all: cargo 4938 passed, clippy zero diagnostics; CMake configure failed (concurrent wrapper run); no ctest baseline credit.
- Focused qbuild quick PASS, 6/6 CMake edges; no Rust files touched.
- Next: immutable guarded original/shadow request with save and 2032/shutdown; inspect counters/failures, then full gate.
- Native replacement disabled; validate image, identity, views and reference ownership.
- Compare Init/Shutdown/message/records, returns/bytes, failures and boundaries on independent instances.
- Unique guarded game labels, 2032/ref0 save/shutdown; wait without qwait.
- Completed ind4-r2-native-92f61086-01: clean 2032-01-05/ref0, full+fast save ok, shutdown suite logged. Four suites, 263052 checks, 262472 reported differences.
- Differences traced to probe counting expected adapter segment-QI negotiation as unexpected (all Read/Write calls), fixed in current code; needs rerun. No semantic conclusion from first counters.
- ff173d78 fixes setup counters, adds outcome totals and local ABI return annotations; focused quick PASS 4/4 edges.
- Outstanding ind4-r2-native-ff173d78-02 (qwait8171), same full/fast saves and2032; immutable DLL SHA256 A52DF4AC728379280876A1C2E1A46C20D6217BED0B9620F252605337AA8DB1CB.
- Full gate rerun64738 pending; previous final run only ret-channel annotations failed.
- DLL immutable build-runs/dll/industrial-r2-92f61086/FreeCity4.dll SHA256 DCFA504194029630A2C1698605FBC8EDC54F4DE43DBDEDC5925708ECDE47C3D7.
- Full gate: cargo4938/clippy0/build PASS; ctest67/68, ret_channel_check needs local [abi-ret] annotations for published AL=1 lifecycle slots (now added). No ABI changes/new questions.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-industrial4-r2-manifest.json.

## Prior round report (superseded scope)

Stopped at the required early boundary audit. The published industrial contract supplies callable interfaces, but does not establish the interception boundary needed to capture the original city/store-owned instance from its first natural Read through destruction. IN3 records the missing contract; this round delivers documentation only.

### Published facts and scope

- Industrial identity: CLSID `0xA990C022`, primary IID `0x477A9E7D`, city component 28, city getter slot 60. The primary has only QI/AddRef/Release/Init/Shutdown; message-target slot 3 and serializable slots 3-5 are published. Successful QI AddRefs, and final Release calls Shutdown then destroys the object (commercial-industrial sections 1, 2.3, 9).
- Public SDK GZCOM creation and those slots permit constructed native probes when the industrial replacement is disabled. That does not observe the city/store-owned instance. With the replacement enabled, the same class request may resolve to our own implementation.
- The industrial owner has no agents or subscriptions (sections 1, 3, 5). Commercial shadow interception cannot supply industrial coverage.
- Foundations sections 3.6-3.7 put the component Read inside class-store load, before the city-init notifications. The public city getter alone does not provide capture of that earlier call.
- Existing plugin factory replacement, commercial shadow, city-detail native acquisition, and public SDK COM/director interfaces were checked within the bounded audit. No published safe forwarder installation/removal contract for the required natural-call scope was established. Native table patching/provider bypass was not guessed.

### Changes and validation

- `b8e853e6`: append IN3 in `docs/specs/questions/commercial-industrial.md`, documenting the exact missing acquisition/interception/lifetime contract and the distinction between constructed and natural calls.
- No core/plugin/ABI/default/map changes. No commercial algorithm changes. No implementation or industrial verification credit.
- No native sessions or game requests were started. No comparison counters were produced; no exactness claim or fraction is reported. Controlled boundary, save/reload, shutdown and reference-warning checks remain unexecuted.
- Cargo, clippy, CMake, ctest and formatting: n/a for this documentation-only early stop. The task explicitly prohibits code-free baseline builds and idle builds.
- Question ID was selected from QUESTIONS.md next-free IN3. Its generated index remains untouched because this lane may edit only its question file under specs and may not run tools/ scripts. Coordinator regeneration is required.
- Initial and post-question Git status were clean apart from this lane documentation before its commit. The question commit succeeded despite a shared packed-refs lock permission diagnostic; no Git configuration/ref maintenance was attempted.

### Next steps

1. Publish an IN3 answer using an already supported public route: exact native identity, interception before first Read, reference ownership, nested-call handling, and removal relative to final release/module teardown. This report requests no new RE work.
2. Regenerate/check the question index through the coordinator authorized workflow; retain the question ID.
3. Once the missing contract is published, authorize a new implementation round with separate native/Rust instances, complete counters, controlled failures, then focused checks/full gate and unique-label guarded save/reload/shutdown runs.

Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-verify-industrial4-r1-manifest.json`. No outstanding requests. Reserved worktree remains on detached HEAD for supervisor publication; queue state is wip, final lane state blocked.
