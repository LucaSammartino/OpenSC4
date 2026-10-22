| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tutorial-contracts4 (TU6-TU9 native contracts) | `re-tutorial-contracts4` / (RE lane, detached worktree) | ready; TU6-TU9 answered in 8bcb132e / 0b4d5b6f / 7ac38403; question check and diff check PASS; docs only, ABI exports unchanged; no game runs | (1) coordinator gate native ABI proposal; (2) glue lane implement global/window binding and native calls; (3) verify tutorial-city reopen and task progress in game |

## Report

The public specification now answers all four blockers filed by glue-tutorial4. Scope is the tutorial system's
external protocol with the original window, not complete UI-class credit or runtime promotion.

- **8bcb132e / TU6:** tutorial.md §4.2.1 proves app-slot49 result identity, GetItem pop4/EAX borrowed item-or-null,
  SetActive pop8/AL changed-state, low-byte active input, ownership and synchronous preferences-cell/save behavior.
  No concurrency guarantee was inferred. Corrected the older three-caller statement: export has four call sites.
- **0b4d5b6f / TU9:** §4.2.2 and §4.4.1 prove heap C allocation100, constructor/base/refcount0, the two explicit
  owner references, no-argument Init and close, uint32 event/pop4/ALtrue, and consumed primary vtable slots6-11.
  Slot10's coordinate arguments are pointers to32-bit values; slot11 returns a signed relative index or -1.
  Reset's owner GUID/task writes occur after the window call, including its -1 result.
- **Critical reopen binding:** original window reads borrowed cISC4TutorialSystem* at global00B20E88. Init calls
  slots7/12/11; task progress calls12/11/20/11. Completion posts the normal end message. A replacement's city
  interface alone does not redirect the global. No extra Init owner/GUID/Lua/flags arguments were invented.
- **0b4d5b6f / TU8:** §4.4.2 proves plain record fields kind+0, ID+4, payload+8/+12. Forwarded payload may advance
  the original task and reenter owner slot20; unconditional no-op handling does not cover this behavior.
- **7ac38403 / TU7:** §4.3.1 proves plain cheat notification+8 string interface, tokenizer and numeric conversion,
  argc3 reset/save/step versus argc2 save/next/quit/reset, exact comparison order, field writes and dispatch calls.
  Added conformance expectations8-10 for reopen, filter routing and cheat arity/order; no code/tests weakened.

**Native ABI proposal:** `build-runs/abi-rows-re-tutorial-contracts4.tsv` is force-added to Git so the coordinator
receives it with the commits. It contains seven direct-call proposals (two registry; window constructor, Init,
close, event, moveToNext), eight consumed vtable rows (1/2,6-11 in the existing vtables.tsv column format), and the
required global-binding note. The direct-calls.md proposals are tabulated table fields because that destination
is Markdown rather than TSV. No docs/abi table or C ABI export changed; no ABI version bump.

**Validation and limits:** `python tools/questions.py index` and `check` PASS; TU6/TU7/TU8/TU9 each resolve only
to answered/tutorial.md. `git diff --check` PASS. Cargo/clippy/ctest n/a (specification and proposal only).
No game runs or shadow comparisons performed. Prior glue-tutorial4 without-window runs are not evidence of a
working tutorial-city reopen; that integration check remains required after implementation. No new open questions.

**Evidence and transport:** read-only re_query export queries first, then targeted main-checkout decompilation,
with instruction bytes/address verification for widths, calling conventions and integer operations. The unnamed
base vtable00A8638C was read directly from the working executable's PE after constructor instructions identified
it (re_query lists only the named +8 table). No REA request was needed. q.py returned Bash E_ACCESSDENIED; bounded
PowerShell reads were used. Commits succeed despite a nonfatal packed-refs.lock permission message; no repair or
shared ref/config changes attempted. Main may advance independently; this detached lane was not rebased/merged.

**Next:** coordinator reconciles the proposed native rows and global binding; glue lane implements these calls
and preserves callbacks; a native/replacement tutorial-city reopen, task-step and close run must verify the result.
The baseline is the reserved worktree commit before 8e6b5033, not the moving main ref. No engine file was read or
edited; no tools, unreconciled files, shared ABI tables, README or HANDOFF were edited.
