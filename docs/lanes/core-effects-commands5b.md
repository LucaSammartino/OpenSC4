| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-commands5b: settled effects script commands | Codex / `core-effects-commands5b` | in progress | Implement settled parser owners and vectors; full gate |

## Handoff
- Goal: close settled effects-parser §§9–13 and effects §19 in core; no ABI/plugin changes or game request.
- Done be2e2dbb: concrete readers, dispatch dictionaries, directory identity and final lifetime; SW/DI/LT/DR fixtures.
- Done 46e94dd6: root owners EC1–50. Done ebd8bba9: definition blocks/effectBase EC51–71, dynamic/sequence/shake/light EC120–160, rotation RO1–6.
- Done 72c9268b: particle scalar/rate/collision/walk/appearance EC72–119. Done 4f911aa1: decal/resource/timed EC161–208 and nested dictionaries EC272–276.
- Done 3ab5fb5e: warp/source/emit/force/tractor EC209–271 and owner table through §19.52. Typed lists reset/clone; EC57 reads inherited tractor end bits.
- Focused final check session 59278 completed: 79/79 parser tests passed. Explicit fmt of all 20 module .rs files passed, zero diff lines.
- Filed EF16 in docs/specs/questions/effects.md: effects §19.52 says instance handler 00596980; effects-parser §6.1 says 0059A730. Retain both literal ledgers, keep only equal-handler claim of EC275 open. Failure evidence build-runs/parser-check-ec275-failure.txt (78/79 before annotation).
- Remaining spec domains: EF15 residual unpublished nested execution, EFP7 numeric qualification, EF16 identity conflict. No new implementation gaps known.
- Current FULL qbuild all is session 82804: workspace Rust tests 6201 passed/0 failed; Clippy failed one needless_range_loop in nested registration TEST. Fixed via iter().enumerate(), explicit fmt nested.rs passed. Preserved failure build-runs/failed-final-gate-clippy-1.txt. Changed source is NOT yet checked by another all; do not claim full pass.
- CMake portion of SAME session 82804 remains pending at C compiler ABI detection (configure log timestamp 19:27:24 local). Collect SAME operation to terminal before corrected all; never launch duplicate. Functions wait cell 55 may be pending on resume. Compiler ID C and CXX succeed; scratch engine/build-x86/CMakeFiles/CMakeScratch/TryCompile-4zlppm contains generated ninja files but no completed ABI result. No process killed.
- Current logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-configure.log. Prior source's gate session 83508 unavailable after restart; it was incomplete (6182 Rust passed, old clippy clean, stalled CMake), not a full pass.
- Round 3 instruction supersedes the native-gate steps above: do not collect session 82804 or run CMake/CTest. Native gate: coordinator, on final tip.
- Round 3 review: existing owner table closes settled rows through effects §19.52 / EC276; README opening now consistently lists EF16 alongside EF15 residual and EFP7.
- Next: run qbuild test, qbuild clippy and qbuild fmt on the 20 owned parser Rust files; record each terminal verdict, then final report/commit/ready merge queue with full HEAD. On guard OVER return continue without queue.
- Renderer/resource/sound remain foreign primitive traits. No lexer; command enter/exit methods return before generic stack pushes/pops. Full EAX fixture inputs remain separate from selected indices.
- Formatting helper build-runs/format-patch.py emits apply_patch input from stdin rustfmt with engine/rustfmt.toml; no filename headers. q.py Bash cannot launch here; use native PowerShell.
