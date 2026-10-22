| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-landfill-records4 (independent landfill occupant verification) | `verify-landfill-records4` / detached worktree (engine lane) | ready: cargo4940/clippy0/ctest68/68; immutable ON/OFF/own-save reload clean2032/ref0; occupant8/9 exact,13/13 destroyed,reload4/4; LG32/LG33 filed; ABI export added; no zone credit | (1) Reconcile LG33 from published contracts. (2) Retain LG31 zone exclusion and LG32 unscored return. |

## Report

Round 2/3 delivers an independent, opt-in native occupant comparison through the published factory, landfill occupant, base occupant and serializable interfaces. LG31 remains excluded: no zone calls, private zone fields or zone verification credit. Starting detached HEAD was c09afb61; no branch/ref/config changes, merges or pushes were attempted.

### Implementation and measured boundary

`FC4LandfillOccupantProbe.h` constructs isolated native objects through GZCOM. The existing `landfill_shadow` switch runs the probe once on the first city day, independently of developer replacement. All final controls use the current40 replacements, including landfill; the developer shadow is inactive. Calls run inside the existing SEH guard. Objects are never Init'ed, inserted into the city or attached to a shared exemplar.

The additive C ABI export `fc4_landfill_occupant_probe` independently constructs the existing Rust `LandfillOccupant` from identical caller-authored record inputs. It returns public query results and Rust-written bytes. Two focused Rust tests verify literal expected bytes, mutations, bounds, bool normalization, rejected versions/truncation and invalid pointers. No core behavior was changed to fit native observations. ABI exports changed; FC4_ABI_VERSION and ABI tables were not changed.

Final real-game result: **8/9 exact**, with the sole compared difference filed as **LG33**. The nine cases cover construction; valid record Read/Write; same-ID SetTileID, orientation and position mutation; empty-byte1 and byte2 normalization; bad outer/base/holder versions; and truncation. Public QI answers and interface identities agree, including unknown-IID refusal. Native AddRef returns2, balancing Release returns1, and final Release returns0. All **13 factory objects were destroyed**: nine primary objects plus four fresh native reload objects. All **four valid record reloads agree**, including their 57-byte records, getters and bounds.

Native captures are compared against independently written Rust records, never supplied to Rust. Native own-output reloads go only into fresh native objects; Rust reloads only its own output. After divergent Read acceptance, subsequent native state/Write is deliberately unscored. Empty own-property/null-parent holders bound these checks: non-null exemplars, changed-ID/shared-exemplar mutation, Init/Shutdown messages, city insertion and class-store registration are not verified. No whole-owner completion or replacement promotion is claimed.

### Questions

- **LG32**, `docs/specs/questions/landfill.md`: the SDK bool for inherited SetPosition reads false despite exact position, bounds and record effects. The published landfill contract specifies these effects but no true-return requirement; Rust's setter returns no value. The return is explicitly logged and unscored. Initial instrumentation's invented true-return assertion was removed, with no Rust behavior change.
- **LG33**, same file: malformed exemplar-holder version1 at offset16 is accepted by native occupant Read (true,39/57 bytes consumed, stream error0), while Rust propagates nested holder failure and returns false. This remains a measured malformed-input difference pending published clarification.
- **LG31** remains the prior zone-boundary blocker, outside this task. No new RE requested. The generated question index was left untouched under the lane's spec-edit/tool restrictions.

### Validation

Final stable `qbuild all` on **17a23477** passed: **cargo4940**, **clippy0 diagnostics**, **CMake4/4**, **CTest68/68** in106.5s, with no tree-changed warning. Subsequent commits change only this lane report. Focused quick: capi529 passed, C++ build passed; rustfmt check passed on both touched Rust files. `git diff --check` passed. Full logs remain in this worktree's build-runs.

The initial baseline all passed cargo4938/clippy0/ctest68/68 but the tree changed during that run, so it was not used as the stable candidate gate. An intermediate gate caught a missing SetTileID return-channel review annotation; a0584dd0 added the published corrected return contract beside the call. Final gate above supersedes that failure. Implementation milestones:31b7506c, c8b59ba7, b2c9800e and17a23477.

### Immutable real-game controls

Final DLL: `build-runs/dlls/occupant-r2-777fe83b2491/FreeCity4.dll`, SHA256 **777fe83b249162b0919fe4b4c6638251430ca6c2be9b3c0a60f2b8ce0b89b879**. All three final controls use this same DLL, current40, enabled reference audit/watchdog/SEH guards, scripted full saves and ordinary quit/Shutdown. No audit suppression. Each label was unique; one request was outstanding at a time; a local waiting helper was used without qwait.

- **vlfr4-r2-off-2032-20261022d**: probeOFF, clean2032-01-01/ref0,138.437s, full saveok1. Own saved fixture `MAIN/build-runs/fixtures/vlfr4-r2-off-2032-20261022d-saved.sc4`, SHA256 **cd1d9ea53d49bf9d3cd29c337879e4ef9f1e8cf36bd219c9d6a844f60cf555ae**.
- **vlfr4-r2-on-2032-20261022e**: probeON, clean2032-01-03/ref0,104.625s, full saveok1 on2031-11-04. Probe8/9 exact (LG33),13/13 destroyed,reload4/4. Own saved fixture `MAIN/build-runs/fixtures/vlfr4-r2-on-2032-20261022e-saved.sc4`, SHA256 **ef3bd25b3077435417b7d18308c01482a4f07b62d8ce0fa189392e5001f3c659**.
- **vlfr4-r2-own-reload-2032-20261022f**: loaded that exact ON-run fixture through the harness (source hash confirmed and preserved), probeON, clean2032-01-03/ref0,18.313s, full saveok1 on2031-11-05. Again8/9 exact (LG33),13/13 destroyed,reload4/4. Reloaded-city save preserved separately with SHA256 **bb6211f4b68ca6c9a4b6e2f9651cf42fdff144be8864f683b63dc71a178a3687**.

Earlier diagnostic/failed evidence is preserved, not discarded:

- **vlfr4-r2-on-reload-2032-20261022a**: clean/ref0/saveok1 on earlier DLL1d523688f69580122d747de55d953b03d4736fc99f8d148b5db0df4b7bdb46bd, initially7/9. The shared saved fixture began2033-12-01, so end2032 was immediate; no six-year control credit.
- **vlfr4-r2-normal-diagnostic-20261022b**: normal-speed confirmation, clean2034-01-01/ref0/saveok1 on diagnostic DLL1c253934cb3868af4e2b3aed1a73f4392279eb652db6196e8b38eb04ed6f3ac7. Same SetPosition-return observation and LG33 acceptance difference; neither is turbo-only. Initially7/9,14 destroyed,reload4/4 before comparator calibration.
- **vlfr4-r2-on-2032-20261022c**: final-DLL startup-stall fallback. Watchdog warning preceded LoadCity completion and probe execution; stall later ended after8063ms. Probe subsequently measured8/9,13 destroyed,reload4/4, but no clean/full-run credit. Label retained; unique retry e above passed with guards unchanged.

No requests remain outstanding. Game files and helper scripts are confined to ignored build-runs locations and are not committed. The ready queue entry delivers this bounded verification instrumentation, not resolution of LG33 or zone completion. Next: reconcile LG33 from already-published contracts; preserve LG32's unscored return and LG31's exclusion.

## Prior round report

Stopped at the task's required initial boundary check, within approximately 15 shell/edit calls. Milestone `b79b2286` records the question and initial handoff. No engine implementation was attempted after the boundary gap was established.

### Published contract findings

- Landfill section 1 identifies `cSC4LandfillZone` as a private plain refcounted object with no GZCOM CLSID. Its documented message-target-shaped vtable provides QI, AddRef, Release, a no-op DoMessage and a destructor slot.
- Section 7.1 documents native acquisition through developer slot 12, `GetLandfillZoneFromCell`: the result is borrowed, with no AddRef. Taking and releasing one's own reference is supported; this does not provide zone behavior calls.
- Section 7.4 explicitly identifies the zone vtable as nonpublic. Sections 4.4.9 and 6.2 describe zone lifecycle and inline serialization, but do not publish callable public zone query/mutation/Read/Write interfaces. Targeted searches found no zone entries in `docs/abi/interfaces.tsv`, `vtables.tsv`, or `layouts.md`.
- Section 9 describes a consumer reading a private region pointer. That is excluded by this task's prohibition on private native fields. Published implementation addresses are not a public calling boundary.
- Occupants have a separate published GZCOM factory/interface and serializable interface (sections 1, 6.4 and 7.3); existing plugin creation uses them and a documented exemplar-holder layout. That does not establish a public zone interface or justify zone-owner verification credit. No occupant-only implementation was substituted for the required initial stop.

### Question and limitations

Filed **LG31**, `docs/specs/questions/landfill.md`, asking for an existing permitted native zone acquisition/calling/lifetime contract or confirmation that public-interface zone verification is unsupported. `docs/specs/QUESTIONS.md` supplied next-free LG31. The generated index was not edited or regenerated: this lane may edit only its question file under specs and may not run tools scripts. No new RE work is requested.

No native calls, comparisons, constructed cases, natural-call interception, fallback/refusal instrumentation or lifecycle/save/reload observations were performed. There are no comparison measurements and no exactness claim. Existing developer 71/71 was not reused as evidence. No raw-record copy, second Rust oracle, host mutation, private-field access, default/config change, ABI change or game request was introduced.

### Validation and operational facts

Build/test/game: **not run**. The task requires stopping on a missing published boundary and prohibits a code-free baseline build. Only documentation changed; no DLL was built or submitted, and no game request is outstanding. The requested clean2032/ref0/save/reload/shutdown evidence therefore remains unavailable.

The first quiet-shell invocation failed with WSL Bash service access denied, so subsequent inspection used bounded native PowerShell commands. Git emitted a packed-refs lock permission error during the first commit but reported successful detached commit `b79b2286`; no shared ref/config repair or maintenance was attempted.

Resume only after LG31 is resolved with an allowed published contract, or the coordinator explicitly creates a narrower occupant-only task. Preserve detached HEAD. Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-verify-landfill-records4-r1-manifest.json`.
