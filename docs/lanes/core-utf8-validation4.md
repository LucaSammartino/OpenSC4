| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-utf8-validation4 (standalone counted-byte validator) | Codex / `core-utf8-validation4` (engine lane; reserved detached HEAD) | WIP: source `6cc9dc9d`; focused 5/0; Rust 5504/0; clippy completed; canonical native gate pending statistics client PID 38168; CTest unrun, 68 configured; ABI unchanged; no questions | (1) Coordinator resolves statistics wait; (2) collect original session 26284 through terminal and full CTest; (3) verify stable source and promote only after a complete gate. |

## Implementation and scope

Implemented the standalone public utf8-validation.md §§2–3 policy in `engine/core/src/utf8_validation.rs`; `lib.rs` has one additive module export. No pre-existing standalone owner or own lane report was found before implementation. Source/test milestone: `6cc9dc9d`; focused-pass record: `362c2952`; pending-gate record: `aaf93300`.

`utf8_is_string_valid(&[u8]) -> Result<bool, UnsupportedCount>` owns the sequential unsigned byte walk. The slice is exactly the readable counted span; prefix callers slice their backing buffer explicitly. Counts above `0x7fffffff` have a distinct admission error. Empty and embedded-zero handling, all lead partitions, full-width guards, ascending continuation checks, deferred special lower bounds, exact advancement and first rejection are implemented directly. Historical five/six-byte forms, surrogates and above-modern-range forms remain accepted as specified. No decoder, normalization, allocation, unsafe pointer access or host oracle is used.

Native null/inaccessible pointers, negative or wrapping count interpretations, native caller/lifetime behaviour and return-register upper bits remain outside this API's evidence. No C ABI, plugin, SDK, default, game or other owner was changed. This is portable standalone policy work, with no native/default/whole-family promotion.

## Verification

- Literal fixture ledger: all public §5's 107 cases across 92 rows, with literal byte arrays and Boolean expectations established before production comparison. Includes all 15 truncations and both rejected sides at all 15 continuation positions.
- Five focused tests passed, zero failed: 107-case ledger, empty/truncation read guards, all 30 bad-continuation stopping traces, ordered lower-bound/stopping traces, and synthetic count-domain admission boundaries without giant allocations. The observer uses the actual production walker with a raw-byte reader. `build-runs/utf8-focused.log` contains the final successful run.
- In-place rustfmt used only the two new owned files. Final qbuild formatter check: exit 0, two files. The module export was not broadly reformatted. Rustup emitted its existing directory-canonicalization warning.
- One canonical `qbuild.py --wt C:/Users/tlori/FreeCity4/.claude/worktrees/core-utf8-validation4 all` was started after stable source and collected repeatedly via the same live exec session **26284**. No baseline/quick/duplicate gate was run, following the task's explicit override. Bash q.py was denied for focused execution; a local quiet Python subprocess runner was used instead.
- All 26 Rust summaries in `build-runs/qbuild-test.log`: passed counts **540, 3, 1, 31, 27, 40, 7, 2, 1232, 3407, 21, 13, 19, 147, 0, 2, 1, 3, 1, 1, 1, 5, 0, 0, 0, 0**; total **5504 passed, 0 failed, 0 ignored**. Clippy completed in 27.30 seconds with no diagnostic failure (`build-runs/qbuild-clippy.log`).
- Native build log ends at **217/217**, after linking FreeCity4.dll and fc4_plugin_load_test.exe (`build-runs/fc4verify-build.log`). **Native wrapper return code, full gate terminal status and COMPLETE are unknown.** There is no CTest result log or temporary CTest output; the configured top-level CTest file contains **68 tests**. This is **not a full gate pass**.
- Before/hold snapshots of 1,327 engine source/config files are byte-identical: SHA256 **c3e22cff564280e4bbc4f3606c63a20c4b84d48fe3e012848501b79654b6a7dd**. Evidence: `build-runs/utf8-source-before.json` and `utf8-source-after.json`; generated abi_entries.inc is excluded. The after snapshot is at the hold point, not after a terminal gate result.

## Waiting client and continuation

Read-only psutil metadata identified this gate's actual statistics client at **2026-10-23 05:48:11 UTC**: PID **38168**, creation epoch **1792733723.8913689**, parent PID **38580**, actual cwd `C:/Users/tlori/FreeCity4/.claude/worktrees/core-utf8-validation4`, argv `C:/Users/tlori/.cargo/bin/sccache.EXE --show-stats --stats-format json`. Parent argv is Python running the approved lane-bin `fc4verify.py --wt <this worktree> build ctest`. Evidence is saved in `build-runs/utf8-stall-evidence.json`; the read-only collector is `build-runs/utf8-cache-client-evidence.py`. A preceding scoped WMI query was denied (HRESULT 0x80041003). No process was terminated and no compiler/cache RPC, daemon, configuration or environment was changed.

Coordinator action is needed to resolve this statistics wait externally, then collect the **existing** session 26284 through terminal and verify all 68 configured tests and COMPLETE. Do not cancel or duplicate the gate. Recheck source stability at actual terminal completion before promoting the WIP queue entry. This lane deliberately claims no native return code or passing CTest result from partial output.

Questions: **none**; the supported normal-domain policy is complete. Game requests/runs: **none**; native fixture executions: **0**. The 107 executed vectors are portable evidence only. No ABI version or C ABI exports changed.
