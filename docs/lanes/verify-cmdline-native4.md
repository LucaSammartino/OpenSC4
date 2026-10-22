| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-cmdline-native4: independent original command-line verification | `verify-cmdline-native4` / (engine lane, detached worktree) | blocked: CL4 owned-original binding contract absent; constructed 0/0, natural 0/0; cargo/clippy/ctest and game n/a; no ABI changes | 1. Publish the CL4 binding contract. 2. Resume independent production comparisons and required game controls only after it is available. |

## Report

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
