| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-cmdline-native4: public native read-only queries | `verify-cmdline-native4` / detached worktree | in progress: round 2, public framework route audit and baseline running | Compare borrowed process getters with production Rust; full gate and immutable 2032 native/current40 controls. |

## Handoff

- Goal: public cIGZFrameWork::CommandLine borrowed getters versus Rust with independently captured GetCommandLineA input.
- Prior milestone: 1c194a5e filed CL4; owned construction/mutators/lifetime remain open and outside this phase.
- Worktree reused detached at 1c194a5e, initially clean; no refs modified.
- Public safe route demonstrated in source by call 15: initialized director FrameWork(), audited framework hold, typed CommandLine(), const borrowed getters, zero borrowed releases.
- Thin CAPI now calls actual production CmdLine; no native output used as parser input. Private exports added, ABI version/maps unchanged.
- Baseline cargo/clippy passed; C++ failed because first hook edit overlapped baseline compilation (missing include then fixed). Retain this attempt; stable full gate still required.
- First fmt attempt used relative paths and was refused as nonexistent; use absolute file paths.
- Implementation committed adb42d1d before call 30. Quick PASS: capi 532 tests, C++ 130/130 edges.
- New CAPI file formatted and qbuild fmt PASS. lib.rs recursive fmt flags 645 pre-existing diff lines in unrelated modules; none changed. q.py Bash unavailable (E_ACCESSDENIED); native rustfmt used only for the new file.
- Stable full qbuild all PASS at 07aa4988: cargo 4963/0/0, clippy diagnostics0, build2/2, ctest68/68 (157s).
- Native-a completed STALLED during full save at2032-01-01: 123/123 query comparisons exact, failures0, ref0, no shutdown proof. Preserve log/verdict; no query mismatch.
- Native-a immutable DLL in build-runs/cmdline4-r2-immutable; source07aa4988, gate57e7b65d. No active request.
- Add per-method positive/negative evidence and register opt-in key (warning fix). No supported watchdog override found; do not alter thresholds.
- Logging/key update92e01745: quick532/0 + build5/5 PASS; stable all cargo4963/0, clippy0, build2/2, ctest68/68 (72.9s) PASS.
- Current40-b CLEAN2032-02-01, ref0, 123/123 exact; positive/negative observed for every search/switch/index method. Full SaveCity(full)11094ms on2032-01-06; PostSystemServiceShutdown reached. No other warnings/INCOMPLETE.
- Immutable-b source92e01745; SHA256053BD7A077A43A31ACB065C3B985D792CD9FEBFADAA82B0E7A343D742621FABC. No engine changes since gate33596de3.
- Native-b STALLED during scripted full save after2032-01-02, ref0, 123/123 exact, no query differences/failures. End-date separation did not resolve Konradshohe native save stall.
- Next: documented Spandau native control cmdline4-r2-native-spandau-2032-c with identical immutable-b DLL/schedule. If clean, match current40 there; no watchdog/save code changes.
- Preserve CL1 extra slot, CL2 empty name, CL3 high bytes; no full 17-method claim.
- Use build-runs/cmdline4-r2-wait.py (never qwait); retain every completed verdict. Upcoming native-spandau-c; current40-b and both native Konradshohe attempts complete.
- Required: immutable native/current40 controls, full save/shutdown to 2032, ref0, supported reload only.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-cmdline-native4-r2-manifest.json.

## Prior round report (retained)

The prerequisite audit stopped before 15 tool calls. No fully documented owned original construction path was established. Filed CL4 in `docs/specs/questions/command-line.md`; production core, CAPI, plugin, defaults and ABI tables are unchanged. The reserved worktree was reused with detached HEAD and an initially clean tree.

### Published facts and precise gap

- `docs/specs/command-line.md` sections 1.1-1.2 provide size `0x3C`, parsing constructor `0x00914E14`, copy constructor `0x00914A30`, nondeleting destructor `0x00419490`, reference count zero at construction, and deleting-destructor/free behavior.
- Section 3 and `docs/abi/vtables.tsv` provide the 17-slot original interface; CL1's slot-16 ambiguity remains open. `docs/specs/framework-rz-files.md` sections 8.1-8.3 repeat layout/constructor behavior without supplying the missing construction binding.
- `docs/abi/allocators.md` documents the global heap allocate/free signatures. This is positive allocator evidence, not proof that arbitrary plugin-owned storage and a guessed constructor prototype are supported.
- `docs/abi/direct-calls.md` documents a default thiscall convention for its listed functions, but contains no command-line constructor/copy/nondeleting-destructor entry establishing their exact call ABI, storage alignment/initialization, or supported destruction sequence.
- The public SDK supplies `cIGZFrameWork::CommandLine()` for the application's process command line and the abstract `cIGZCmdLine` interface. No documented owned parse/copy factory or creation CLSID was found in the audited specs/ABI/SDK declarations. The SDK's switch-overload order differs from the original slot table; it was not used as an original dispatch binding.

### Measurements and retained limitations

- Constructed original comparisons: **0/0, not exercised**. Natural original comparisons: **0/0, not exercised**. No exactness or coverage credit.
- Acquisition attempts refused before execution: direct constructor/copy calls without the complete typed/storage contract; mutating the borrowed process object; inventing a factory/private allocator; substituting an SDK implementation or a second oracle. No runtime acquisition failure is claimed.
- Native differences observed: none, because no native calls were made. Fallbacks/resynchronizations: zero. Original references acquired/released: zero/zero; in-game reference warnings were not measured.
- Game requests: none outstanding or completed. No 2032 own/native/current40 controls, save/shutdown or reload ran; prerequisites were unavailable. No game files were accessed.
- Cargo/clippy/ctest/fmt: not run; this is a question/report-only change. The task-specific instruction forbids spending the unchanged baseline/build budget when binding is unavailable. No engine implementation or exports changed.
- CL1 slot 16, CL2 empty switch name, and CL3 high-byte/locale questions remain unchanged. No speculative core correction was made.
- The generated questions index is intentionally untouched: this lane may edit only its question file under `docs/specs/` and may not run `tools/` scripts. The coordinator can regenerate the index through its normal permitted workflow.

### Next steps

Publish CL4's exact construction/copy/nondeleting-destruction ABI and storage/lifetime contract, or a supported original owned factory/parse/copy service. Then compare actual production core/CAPI with independently supplied ASCII ordinary/quoted/slash/colon fixtures and safe insert/erase/join/copy/lookup operations, retain every difference/refusal, run focused checks and one final full gate, and perform uniquely labelled 2032 controls with save/shutdown. Lane remains blocked; any merge-queue record is `wip` and carries no verification claim.
