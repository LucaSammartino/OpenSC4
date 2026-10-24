| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-au40 (AU40/AU44 owner contracts) | Codex / `re-audio-au40`, detached HEAD | ready: partial closure; AU40/AU44 answered, 18 spec vectors, doccheck passed; no ABI change | AU41–AU43 remain open for a successor |

## Report

Partial closure under the requested two-thirds wind-down rule. Answered AU40 and AU44 in
`docs/specs/audio.md`, corrected their summaries in `audio-owner-contracts.md`, moved both
original questions to `questions/answered/audio.md`, and regenerated `docs/specs/QUESTIONS.md`.
No duplicate open questions were added. AU41, AU42 and AU43 remain open, without a closure claim.

- AU40 (`1a339eeb2`): listener geometry, sea-plane handling, normalization/rounding, zoom/tuning
  sizing, city/cheat payload gates and service/message failure lifetime; ten conformance vectors.
  Notification movement uses the camera, while its payload points to listener position.
  Missing server, null message allocation and some missing/short tuning arrays are native
  invalid accesses, not recoverable false results. Low-zoom dimensions cross foreign city/renderer IO.
- AU44 (`6ee211ad9`): all five interface views, defaults, fire predicate, message/scan paths,
  group registration and nontransactional lifetime; eight conformance vectors. Failed Init
  retains subscriptions. Shutdown never removes the scape group. Message insertion/removal
  keys use the acquired position interface; enumeration uses the visited occupant pointer.
- Evidence: main-checkout re_query and class decomp only, with numeric/width claims checked
  against instruction bytes and constants read from the working executable. No REA call was
  attempted because the session has no binary open. No assembly or pseudocode copied to specs.
- Gate: foreground `python build-runs/check-docs.py` completed `doccheck check: ok`;
  log `build-runs/doccheck-re-audio-au40.log`. The earlier q.py launcher failed in Git Bash
  CreateFileMapping before starting doccheck; that is distinct from the passing Python run.
- Cargo, clippy, CTest and game runs: n/a for prose-only RE work. No ABI export changes,
  proposed ABI rows, engine reads/edits, plugin edits or coverage-map edits.

## Handoff
- Guard measured 105,664 / 150,000 after the AU44 milestone; stopped new RE work at wind-down.
- Next: AU41 SEM constructors/registration and model slot 5/6 arithmetic, then AU42 scape
  remaining slots/resources/smoothing/timers, then AU43 preferences dialog/song persistence.
- Owner closure table was read from the core-audio-close13 worktree's lane file; main has no copy.
- Source evidence remains in main `decomp/` and `ghidra/export/windows`; local helper files/logs
  are ignored under this worktree's `build-runs/`.
- Detached HEAD is intentional; supervisor publishes the commits to reserved branch re-audio-au40.
