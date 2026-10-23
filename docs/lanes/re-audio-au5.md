| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-au5 (audio handler AU13–AU16) | `Codex` / `re-audio-au5` (RE, detached worktree) | ready: AU13–AU16 answered; 17 conformance groups added (42 total); doccheck passed; no ABI changes or game runs | Implement the resolved owner algorithms and rerun owner closure tests. |

## Report

Resolved AU13–AU16 in `docs/specs/audio.md` §§7.1.1–7.6 and moved the original questions, with dated
DIFFERS answers, to `docs/specs/questions/answered/audio.md`. Regenerated `docs/specs/QUESTIONS.md`.
No new questions were filed. These answers remove the four specification blockers recorded by
`core-audio-events5`; implementation and owner closure verification remain for the engine lane.

- AU13: completed dispatch table, queued station messages, radio selection/settings, construction
  sound IDs and positional calls, dispatch-category/capability notifications, modal nesting and mouse bounds.
  Slot10 is a raw data1 getter with no acquisition; the handler releases its returned pointer, not the
  message. The mouse route can return false through main DoMessage. Subscribed C53D10AA/E94671ED are ignored.
- AU14: constructor defaults, timer/list initialization, AudioScape service acquisition, ignored service
  return values, checked versus unchecked lookup failures, repeat Init/Shutdown and the lack of rollback
  when AudioScape lookup fails after event registration. Shutdown releases/nulls radio, without reacquisition
  on later Init. The destructor is separate from Shutdown.
- AU15: alternate automata selectors, all six derived sound IDs, positional range 50.0f, global generated
  ID use and four-word event packing. Ordinary play rejects failed position QI; its coordinates truncate
  toward zero. Apparent getter arguments in the earlier spec were arguments to the following event post.
  EA3ADA33 failed position QI continues with indeterminate coordinates; no zero fallback was invented.
- AU16: DoEvent callback table and returns, main-handler torch ownership, sound/key/volume rules,
  process-global history, numeric conversion/order and destruction. Native list destruction frees raw
  modal nodes without releasing their retained window pointers.

Added conformance groups 26–42 in the owning spec: lifetime failures/retries, message and reference traces,
sound-selection matrices, inclusive modal edges, QI failures, packed coordinates, torch transitions,
timestamp wrapping, numeric examples and destructor behavior. They are spec vectors, not engine code.

Evidence came from the main checkout's read-only export and selected decompiled functions, with instruction
bytes checked for numeric operations, offsets, argument order and return paths. The local ignored helper
`build-runs/peek-audio.py` read PE constants, radio station strings and message getter addresses; no game
material, pseudocode or assembly was added to tracked files. REA was unnecessary; no REA error/fallback.

Validation: `python tools/questions.py index` succeeded; `python tools/doccheck.py check` passed;
`git diff --check` passed. Cargo/clippy/ctest and game/shadow runs are n/a for this spec-only change.
No edits to engine/, plugin, capi, tools/coverage-map.tsv or docs/abi/. No proposed ABI rows or C exports.
Git printed a packed-refs.lock permission diagnostic during commits, but both commits completed normally
on detached HEAD; no shared refs were changed.

Milestones: `3e2718e2` (lifetime rules), `61f7245c` (complete answers, question movement and vectors).

## Handoff
- Goal complete: AU13–AU16 answered with address-backed prose and conformance groups.
- Done: 3e2718e2 and 61f7245c; doccheck passed and question index regenerated.
- Next: coordinator publishes detached HEAD to reserved lane branch and lands spec; engine owner implements it.
- No pending game runs, ABI proposals or questions.
