| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lua-shadow5: bounded Lua collector/registration shadow | Codex; glue-lua-shadow5 (detached lane worktree) | ready: lua-registry 73/73 exact to 2032; full replay open LH6-LH7 | Land measured registry work; resolve interception contracts before full Lua replay or coverage credit. |

## Changes and scope

`lua_shadow = 1` enables a default-off, record-only `lua-registry` comparison at
city load and monthly. It reads the original immutable CRT extension collector
using the layout documented in advisors.md §8.2 (bounded ReadProcessMemory,
rebased from the preferred image address). It compares all 47 ordered names,
isGlobal bytes and non-null callback bindings against the production core's
collector. Unspecified structure padding is not a field and is not compared.

The original collector's callback addresses are opaque foreign identity inputs.
A private static-link adapter executes the production core's
`collectors::register_api(true)` followed by `register_api(false)` in a local
stack/table model. It compares the resulting global/game callback addresses and
`_tf_` callback address against the original app-owned VM. Original values are
read with public raw table operations, without invoking metamethods, scripts or
callbacks. The native stack is restored and its balance checked; the replay also
checks stack restoration after each registration pass. This checks callback
entry-point identities, not Lua closure heap identity/upvalues or callback bodies.
The original collector and registered values are recorded in the game log at
each sample. There are no native dispatch hooks, service/factory substitutions,
retained game references, or public DLL C ABI export changes.

**This is bounded registration-state verification, not full Lua verification.**
Every chunk, game-driven global/table operation and script call is **not**
recorded/replayed. There is no `lua shadow` success line and no claim to verifying
the complete 47 KB Lua host. The separate `lua-registry` label prevents the
bounded result from masquerading as that coverage.

## Validation

- Focused Rust tests: 4 passed, including wrong callback binding, changed name,
  flag, scratch value, missing entry, null callback, unterminated name and invalid
  private transport inputs. Owned-file qbuild fmt and git diff --check passed.
- Final source `9e2b85dd3`: required `qbuild all` PASS. Workspace **6807 passed,
  0 failed, 0 ignored**; clippy **0 diagnostics**; native build **47/47 edges**;
  CTest **70/70**, 88.8 s. Native steps used the coordinator queue through the
  same qbuild operation (`qbuild-20261024-014147-build-ctest`); no direct MSVC
  configure/build/ctest and no duplicate native request.
- Initial all gate built successfully but passed CTest 69/70: ret_channel_check
  required an explicit citation for the IsCFunction AL bool result. Fixed with
  the lua-host §5 slot 16 citation. Focused checker then had 0 flagged uses;
  repeated full gate passed. Full initial logs preserved in
  `build-runs/initial-{fc4verify-build,fc4verify-ctest,qbuild-test,qbuild-clippy}.log`.
  Final logs use their ordinary names in build-runs. An earlier q.py focused
  test launch failed Bash CreateFileMapping; its log is
  `build-runs/q/20261024-012649-7620.log`; direct PowerShell cargo succeeded.

## Real-city result

[Run 349](../runs/lua-registry-shadow5-9e2b85d-turbo-2032-361.md), label
`lua-registry-shadow5-9e2b85d-turbo-2032`, Konradshohe, turbo, `lua_shadow=1`,
requested end 2032-01-01, actual verdict date **2032-01-03**: **clean**,
`ref_warnings=0`, foreign exceptions/watchdog stalls/rtc/overruns all 0.
DLL SHA256 prefix `608b8211`, city time 47.156 s, queue wall time 66 s.
One blocking directory-notification wait collected the unique verdict, without
qwait or polling; no game/native requests remain outstanding.

Exact measured count:

```text
lua-registry 73/73
```

All 73 dated lines (load 2026-01-27 through month 2032-01-01) have 47 entries,
collector_diffs=0, binding_diffs=0, scratch_diffs=0, stack_diffs=0,
capture_diffs=0, mismatches=0. Total: 3,431 sampled entries/bindings.
Scratch callback was `0x0040C890` in every sample. No normal-speed rerun was
needed because no turbo differences occurred. The harness summary does not
automatically list this new label; the count above is from the dated log lines.

## Exclusions, questions and next steps

No nondeterministic fields are excluded from the bounded comparison. Broader
VM execution is unmeasured, rather than treated as matching or excluded.

- **LH6**, docs/specs/questions/lua-host.md: the spec establishes no supported
  pre-initialization observer installation covering all Lua instances, cached
  interfaces and subsequent calls/internal table accesses. Full interception
  remains unimplemented pending that contract.
- **LH7**, same file: full replay needs a specified treatment of foreign inputs
  and nondeterminism. No speculative replay exceptions were added.
- Coordinator: regenerate the questions index (engine lanes cannot run tools/),
  and assess bounded collector/registration coverage separately. Do **not** mark
  full Lua host verified or append lua_shadow to the full shadows regress profile
  on the strength of this run. No tools/ file was read or edited.
- Implement complete chunk/call/state recording and replay after the contracts
  are supplied, then repeat real-city validation for the full `lua` label.

Commits: `6c5fad877` questions, `33cacb5aa` implementation, `9e2b85dd3`
return-channel citation; `c0dafa2ef` / `e61f825be` gate milestones.
