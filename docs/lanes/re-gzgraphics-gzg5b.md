| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzgraphics-gzg5b: GZG1–GZG5 behavior closure | Codex / `re-gzgraphics-gzg5b` (detached HEAD), RE | in progress: round 2 native-evidence audit; GZG4 already answered | Resolve remaining boundaries and queue answered work READY, listing residual questions |

## Report

Completed **GZG4 only**. The requested five-question closure is incomplete and
must not be treated as ready for the blocked core owner boundaries. The brief's
two-thirds research allocation was reached during this round; the remaining
budget is reserved for validation and a durable handoff, rather than asserting
unverified behavior for the other four questions.

### Changes and evidence

- `475e79821`: extended gz-graphics §§7.1–7.4 with native realization/release,
  failure cleanup, adjusted interfaces, retained native listeners and Windows
  keyboard, character, mouse, wheel, activation, resize, paint and close handling.
  Corrected primary slots 7/8 (suspend/resume), 15 (repaint event) and 18
  (keyboard filter/repost). Stored subtype-13 words are signed client x/y pixels.
- `9cf994fef`: final instruction review followed slot 21 through complete +24's
  destructor thunk: it shuts down and deletes the canvas, independent of reference
  count. This corrects the first milestone's erroneous framework-exit reading.
- Added **17 declarative vectors, GZG-V62–V78**. They cover native failures,
  reverse-registration listener order, coordinate clamping and feedback, key
  flags, signed byte text, wheel scaling, the 1025th-message loss and deletion.
- Moved GZG4 verbatim to `docs/specs/questions/answered/gz-graphics.md` with
  ANSWERED/DIFFERS and regenerated `docs/specs/QUESTIONS.md`.
- Export queried first. Evidence uses main-checkout decomp and paged instruction
  bytes; no REA request was made because the brief says no binary is open there.
  Local extraction helper is ignored `build-runs/read_re.py`; no RE output is
  committed. No engine, plugin or capi file was read/changed; ABI tables and the
  coverage map are unchanged.
- The core owner table lives under forbidden engine/. Used its mirrored open
  section list in the main-checkout `docs/lanes/core-gzgraphics5.md` report.

### Validation and limits

- `python tools/questions.py index`: PASS.
- `python tools/doccheck.py check`: PASS after the final semantic correction.
- Cargo, Clippy, CTest: n/a, spec-only. No game runs, screenshots or shadow
  comparison. The vectors are expected contracts, not measured game results.
- GZG1, GZG2, GZG3 and GZG5 remain in the open question file. No new question
  IDs or proposed ABI rows; C ABI unchanged. The existing GZG6 answer is preserved.
- Both commits succeeded despite Git reporting that shared packed-refs.lock
  could not be created. No shared refs/config were modified or repair attempted.

## Handoff

- Round 2 resumed 2026-10-24: READY is authorized for answered work even with residual questions.
- Round-2 geometry/stretch milestone: §§3.2.2–3.2.5 and 6.3.1, twelve new vectors V79–V90; corrected directional line cap, settled chain/fan polygons, cubic overshoot and stretch mutations.
- Native numeric instructions checked; GZG1 residual is non-AA outline quads and general line ties/clipping; GZG5 residual is standalone kernels/formats/locks.
- Goal: answer GZG1–GZG5 in gz-graphics.md with behavior and vectors; no engine edits or reads.
- Done: GZG4 milestone `475e79821`; §§7.1–7.4, answered question, regenerated index; final review adds deleting-slot correction and vector V78.
- Owner closure: core lane report mirrors open rows for §§2–7; actual table is in engine/, forbidden by task, so use mirrored list.
- Next: query GZG1 helper callees, then read remaining geometry helpers in main decomp/_unnamed/region_00820000.c by address/offset.
- Evidence: main checkout C:/Users/tlori/FreeCity4; re_query.py first, then decomp/; REA unavailable by brief.
- No game runs, ABI edits or proposed rows.
- Canvas corrections: primary 7/8 suspend/resume; 15 repaint event; 18 keyboard filter. Native listeners register at front (reverse registration order).
- Canvas quirks: realized byte set on failed slot-4 realization; keyboard filter consumes an uncounted 1025th record.
- Final review: slot 21 invokes deleting destructor via complete +24, NOT a framework exit; fixed before final report.
- Remaining GZG1: 0082CC60 outline cubic conversion; 0082D040 fixed-point line endpoint/ties/clipping; polygon closure/coverage helpers around 0082E130–0082FC30. Prior partial ellipse/blit/gradient rules are already in spec.
- Remaining GZG2: hardware extensions/base, exact source/gradient subdivision and AA coverage; SimGL 009922D0/00992F80 require assembly (export pseudocode missing).
- Remaining GZG3: unnamed graphics IO, driver discovery/INI precedence, init rollback, screen formats, projection and presentation; decomp cGZGraphicSystem.c and _unnamed/region_00980000.c.
- Remaining GZG5: stretch clipping and standalone kernels/formats/lock contracts; ordinary clipping and indexed-loader quirks were settled by prior lane.
