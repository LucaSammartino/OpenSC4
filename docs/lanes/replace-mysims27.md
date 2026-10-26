| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-mysims27 (engine, MySim replacement safety) | Codex / `replace-mysims27`, detached HEAD; supervisor publishes branch | continue; code milestones `624ddff6c`, `98b01f818`, `fc7526e1b`, `b1ba1ffb7`; C ABI adds 5 exports; no seam reservation; no game evidence | 1. Complete gate on final source. 2. Run the three game proofs. 3. Enqueue after acceptance. |

- Status: mysims is not yet ready for the replace list; no merge-queue entry or enqueue was written.
- Criterion 1: compiled-order 69-slot cISC4MySim front added for MySim/MyStreetSim; advisor identity/refcount shared. C++ factory/ABI tests added, awaiting CTest.
- Criterion 2: advisor-system Write/Read forward to existing codecs; live dispatch saved/owned and restored dispatch bound at startup. Real save/reload not run.
- Criterion 3: constructed advisor core published before Init; pending factory handle transfers to first QI without a hidden extra reference.
- Measured Rust: interrupted all attempt has CAPI 648 passed/0 failed, including five pre-Init factory objects and MySim record roundtrip (`build-runs/qbuild-test.log:30`, `:41`, `:658`).
- Final personal-name persistence assertion and final plugin startup binding patch need the next gate; the interrupted all attempt began before those edits.
- Criterion 4: Lua tables, subject/string/number fields, context activation and tuning_constants reader connected per mysims sections 4.4/4.11; live tuning unmeasured.
- MySim state getters use RwLock storage. UI controls/audio/camera/balloons remain delegated by section 0; move_in_control is still unwired. Script action supplies move-in for persistence validation.
- Gate: no all PASS. First all attempt saw missing StateLock::get_mut (fixed), then was interrupted; corrected-source all was interrupted at context wind-down after Rust compilation/testing began. No running exec sessions remain.
- Earlier quick's 5917/0 Rust results predated the final state-lock source; dependent/native build failed on get_mut (`build-runs/fc4verify-quick-deps.log`), now fixed. They are not a final gate.
- Format: core/src/mysims/sim.rs and capi/src/advisors_tests.rs pass; capi/src/advisors.rs has pre-existing tail formatting differences left unchanged (qbuild fmt reports them).
- 2032 house replace: not run; exact default list requested asynchronously because tools/regress.ps1 cannot be read/run under this lane's clean-room rules. No answer received.
- Save/reload: not run. `build-runs/replace-mysims27-save.script` moves in FC4 Persist, queries, saves; reload script queries. Action acquires cISC4BuildingOccupant and logs id/name/slot/state/age/wealth/generation/home/bio/portrait.
- Shadow: not run; required mysims 73/73 exact still pending. ref_warnings: unmeasured. No game requests outstanding.
- Questions filed: none. Do not claim readiness from earlier lane results.
- Next: qbuild all on stable final source; resolve failures before game. Use main build-runs/game-queue, own DLL via main-relative path, one request at a time, new labels and qwait.
- Next: prove native early factories return objects; run native mysims shadow to 2032; export a scripted save and reload that slot copy; run exact supplied house list plus mysims to 2032 with ref_warnings=0.
- Next: verify remaining UI move-in-control integration and dispatch tuning coverage against permitted specs; keep unsupported UI behavior explicit. Enqueue only after the full gate and all three required measured proofs.
