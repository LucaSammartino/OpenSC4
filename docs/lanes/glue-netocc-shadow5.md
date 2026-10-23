| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-shadow5: network occupant record shadow | Codex / glue-netocc-shadow5 | in progress | Implement normalized shadow, full gate, city through 2032. |

## Handoff

- Round 3 resumed at 79619a24; no outstanding operations. Manifest: main build-runs/run4-glue-netocc-shadow5-r3-manifest.json.
- Confirmed network-occupants §4.6: attachment selection uses exemplar resource instance, not model property.
- Next: derive pole host input from decoded base.holder.parent_key().instance; regression, full gate, unique city request to 2032.
- Implemented decoded exemplar-key host input; removed model-property QI from native shadow and private transport argument.
- Focused capi shadow tests: 3 passed (new regression covers all four valid subtypes with unrelated model property).
- Explicit capi/src/netocc_shadow.rs fmt PASS. Initial fmt path engine/capi/... was invalid, corrected to engine-relative path.
- q.py focused invocation failed to start Bash (CreateFileMapping error 5); direct cargo focused tests passed.
- Next: commit implementation, run one stable-source all gate, submit netocc-r3-2032-1.
- a58b9739 implements the resource-key fix. Full gate: cargo 6234 passed, 0 failed/ignored; clippy 0 diagnostics.
- Native build/CTest queued as qbuild-20261023-221501-build-ctest (same all operation, session 17278); collect terminal result.
- Added missing prior run record docs/runs/netocc-r2-2032-2-323.md; documentation-only changes while native gate waits.
- Round 3 full gate terminal PASS: cargo 6234/0/0, clippy clean, native 103/103 edges, CTest 68/68; qbuild-20261023-221501-build-ctest.
- Next/outstanding: netocc-r3-2032-1 city request to 2032-01-01, turbo=1, netocc_shadow=1. Wait using build-runs/wait-netocc.ps1, never qwait.
- Round 2 uses detached HEAD; commit normally without moving refs.
- Prior milestone 2b25736f: path key codec fix; be4072e6: full gate PASS.
- NO45 resolved by coordinator: normalize only flags E000, triangle alpha FF, transform mask 7 to 5 at scale 1.
- Next: private record comparison transport, default-off plugin capture at load/month, focused tests.
- Implemented private numeric-token record comparator and default-off plugin load/month hooks.
- Focused normalization/truncation tests: 2 passed; NO13 power-line record test: 1 passed.
- NO13 fixes v2 power-line layout (position, cells, boolean, two refs), rejects other versions.
- Formatting applied via apply_patch generated from rustfmt check hunks, no source stdout redirection.
- No game requests outstanding. Manifest: main build-runs/run4-glue-netocc-shadow5-r2-manifest.json.
- 7e82f20c implemented shadow and NO13 fix. Full gate in progress on that source:
  workspace 6233 passed, 0 failed/ignored; clippy exit 0 diagnostics 0.
- Full gate terminal PASS: native 158/158 edges, CTest 68/68; request qbuild-20261023-205539-build-ctest.
- Next: submit unique netocc-r2-2032-1 game request, wait with local event waiter (not qwait).
- netocc-r2-2032-1 completed clean to 2032-01-01, ref_warnings=0; shadow 0/73 (361 mismatches/line).
- First failures are native capture refusals: WriteCapture rejected property variants. Added opt-in traffic-shadow variant capture.
- Corrected pole model lookup to shared verified property helper; improved per-class diagnostics and unknown load date.
- Next: full gate on corrected capture, then unique netocc-r2-2032-2 game request. No outstanding game requests.
- Corrected-source gate PASS: cargo 6233/0, clippy 0 diagnostics; native 75/75 edges, CTest 68/68.
- Native request qbuild-20261023-211011-build-ctest completed; tree-change note is documentation commits only.
- netocc-r2-2032-2 completed CLEAN to 2032-01-01, ref_warnings=0; harness reports netocc-r2-2032-2 0/72 dated lines.
- Including unknown-date load: 0/73 actual comparisons, each has 3 mismatches (all poles); 1280 other records exact.
- No outstanding builds or games. bbdb3554 engine source passed full gate: cargo 6233/0, clippy clean, native 75/75, CTest 68/68.
- NEXT: fix pole host input: §4.6 says exemplar RESOURCE KEY, but ModelInstance reads model PROPERTY 0x27812821.
- Observed wrong input 0x0AD30000 has subtype zero; core returns AttachmentInvariant at byte 1163/1193 (360 bytes remain).
- Likely use decoded OccupantBase.holder.parent_key().instance; confirm API and §4.6 before editing, never force a table.
- Add focused regression, explicit-file fmt, full gate after diagnostic source change; next game label netocc-r2-2032-3.
- Second run record still needs creation (next available run number was 323): DLL d271da95, real_seconds=62.125.
- Evidence build-runs/game-results/netocc-r2-2032-2.log/.verdict; monthly fixups=857, records=1283, bytes=478251.
- Guard context OVER: continuation required, no merge-queue entry. Keep default-off and NO45 normalization unchanged.
- Event waiter: build-runs/wait-netocc.ps1. Never discard an existing unique-label verdict.

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
