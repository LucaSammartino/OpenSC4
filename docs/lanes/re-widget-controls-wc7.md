| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widget-controls-wc7: control behavior questions | `Codex` / detached reserved worktree `re-widget-controls-wc7` (RE lane) | ready: WC1–WC6 answered; 659be86a3 WC2, 595992395 WC3; doccheck PASS, whitespace PASS; 25 new vectors; no ABI/game changes | Coordinator lands queued HEAD; engine lane implements/tests settled contracts and updates closure rows. |

## Report

Final round completes the existing WC2/WC3 TreeView work. All WC1–WC6 are
answered in the archive; no new owner or question was started. Specification
closure does not claim that the engine owner is implemented or game-verified.

### Changes and measured evidence

- `659be86a3`: completed section 6.6 WC2 primary adapters, image/insertion
  wrappers, move/removal, enumeration and ownership/failure contracts; T27–T36.
  Images retain buffers, fonts are borrowed, arrays have no capacity argument.
  Positive move references insert after, absent ones wrap to the beginning;
  move has no cycle guard. Root removal clears descendants but preserves root.
  Image mutation can grow metrics without invalidation; resource failure does
  not produce an aggregate Init rollback. WC2 was moved to the answer archive.
- `595992395`: section 6.7 WC3 input/drag/rendering and T37–T51; refreshed
  inventory/coverage and archived WC3. Empty-tree navigation is unsafe; page
  movement uses ceil row count; Control toggles only with flag 0x10 and Shift
  has no range policy. Timer/rectangle state is shared across instances. Drag
  has a synchronous acceptance byte and borrowed helper data, rejects dragged
  ancestry, emits a drop pair but does not move nodes, and cancels on empty
  target selection. Selected decoration is two flat fills, with integer RGB
  average and a rounded quarter-height band, not interpolated gradient pixels.
  Expander strokes, connector geometry, access failures and clipping delegation
  are specified. `009D2577` is MouseMove, not paint; GZPaint is `009D37C6`.
- Earlier work retained: `14590ec17` WC1/L27–L36; `391e9d561` partial WC2/
  T19–T26; `2cc7327cc` WC6; `819363eb7` WC4/WC5. No duplicate question added.
- Evidence: read-only main-checkout re_query export and bounded decomp reads
  of the two named TreeView owners. Numeric widths, signed/unsigned clamps,
  shifts, timestamp subtraction, average/quarter-band arithmetic and expander
  parity were checked against instructions and cited in the spec. REA was not
  invoked: the brief explicitly says no binary is open. q.py's Bash wrapper
  failed CreateFileMapping; bounded native PowerShell reads worked.
- Twenty-five new vectors this round (T27–T51) are implementer scenarios,
  not executed comparisons. No code, engine, plugin, capi, ABI-table or
  tools/coverage-map.tsv changes. No proposed ABI rows file is needed.

### Gate and limits

Final round ran `python tools/doccheck.py check` exactly once: PASS, exit 0,
`doccheck check: ok`. Log: `build-runs/doccheck-re-widget-controls-wc7-r3-final.log`.
`git diff --check`: PASS, exit 0; log:
`build-runs/diffcheck-re-widget-controls-wc7-r3-final.log` (CRLF advisory only).
Cargo, clippy and ctest: n/a for spec-only work. Commits succeeded despite
Git's packed-refs.lock permission message; shared refs/config/hooks were not
changed. No game run, shadow exactness result, game request or C ABI export change.

### Questions and owner closure

WC1–WC6 are archived in `docs/specs/questions/answered/widget-controls.md`.
No open entry remains in `docs/specs/questions/widget-controls.md`.
`python tools/questions.py index` was run after each archive move; its
generated index was unchanged. No new question was filed.

The expressly requested engine README closure table was read only. An engine
lane can now remove WC2 question boundaries from TreeView rows 6.1–6.3,
WC3 from 6.2/6.4/6.5 and WC6 from 6.5, plus all three from row 8, **after
implementing and testing** these contracts. The root-inclusion proviso below
that table is stale: WC6 already settled flag-controlled inclusion. ListBox
WC1 boundaries have section 5.6 and vectors for implementation tests. Grid,
TextEdit and FileBrowser retain their own linked owner-spec questions; no new
closure of those separate questions is claimed. No engine file was edited.

Exact final raster pixels (glyphs, triangle sampling, line/rectangle edge
coverage, graphics clipping) belong to the invoked service contracts. Modal
termination policy belongs to the enclosing runner; the TreeView owner
checks live target selection rather than a modal return code. Neither is an
unanswered TreeView policy question or a game comparison claim.

## Handoff

- Goal complete: existing TreeView questions answered; no new RE work queued.
- Milestones: 659be86a3 WC2; 595992395 WC3; earlier milestones above.
- Gate: final doccheck and whitespace checks passed, exit 0 each.
- Next: coordinator lands full HEAD from ready queue; engine lane
  implements/tests the settled boundaries and updates its table.
- Native unsafe allocation, empty-key navigation, cycle requests and stale-root
  restart are deliberate spec quirks, not omissions to replace with guesses.
- No game-request labels outstanding; no ABI rows file or export changes.
- Detached HEAD is intentional; supervisor publishes the actual queued HEAD.
