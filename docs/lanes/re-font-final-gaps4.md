| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-font-final-gaps4: resolve FN15-FN17 | `re-font-final-gaps4` / (RE spec lane; detached HEAD) | ready; docs only; FN15-FN17 answered DIFFERS; questions and whitespace checks pass; ABI unchanged; no game runs | Implement the proved contracts from drawing-fonts §7/§9; coordinator may queue this docs change. |

## Result
The three named blockers are answered in [drawing-fonts.md](../specs/drawing-fonts.md), with exact instruction
anchors and discriminating vectors. Implementation is outside this lane. No whole-owner implementation,
shadow, replacement or runtime-verification credit is claimed.

- FN15: published [font-name-map-fn15.tsv](../specs/data/font-name-map-fn15.tsv), all 477 first-wins mappings from
  479 source records. Section 7 includes executable hash, PE VA/file offsets, record layout, extraction order,
  duplicate ordinals, canonical TSV hash and alias/fallback vectors. No Adobe-list substitution is assumed.
- FN16: proved explicit startup PC24 via 0x0045236C → 0x005F85E0 → 0x008929E0. Font arithmetic inherits the
  thread's current x87 controls. Documented the two multiply operations before truncation; default f64 is wrong.
  Corrected the existing spec's narrow-band non-space branch: it bypasses scaling (8 px stays 8 at s bits
  3F7D70A4). Five numeric vectors distinguish precision, branch selection and the space adjustment.
- FN17: explicit Lock/Unlock return true on normal completion, ignore renderer failure, and change the counter
  without rollback. Unlock at zero wraps to FFFFFFFF. Null renderer on a callback edge faults before the count
  changes; nesting without a callback succeeds. Eight state/return vectors cover these cases.
- Moved all three original entries, with provenance and dated DIFFERS answers, to
  [questions/answered/drawing-fonts.md](../specs/questions/answered/drawing-fonts.md), then regenerated QUESTIONS.md.
  No new open questions; drawing-fonts now has none.

## Evidence and validation
- Work commit: `0d9d496d`. The earlier preserved `9daff95a` contained only a handoff note; no partial spec/code
  was used. Existing reviewed drawing-fonts and its question files were identical between that base and
  main `94eeb430`. Read main's new glyph-textures.md narrowly: it delegates font rendering and proves none
  of these three contracts.
- Static export queried first, then existing cGZFont decompilation used for orientation. Numeric and return
  rules verified against instructions. Independently compared 324 export instruction byte records with
  main's bin/SimCity4.exe: all matched. No REA call was necessary.
- Executable SHA-256: `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
  Published table equals the full extraction after unique insertion: 477 keys, only two duplicates.
  Canonical TSV SHA-256: `421d2a61eab4f96e50a94aee1980bdcbfc718f7b2e147b34fbc262172345ecae`.
- Native x87 arithmetic measured for four input pairs under PC24/53/64. In particular adv=±65535,
  s bits 3F800080 gives ±1 at PC24 and ±(1−2^-32) at PC53/64, truncating respectively to ±1 versus 0.
  This is hardware arithmetic evidence, not a game run. Precision is an inherited input, not a promise
  that all possible callers retain startup PC24.
- `python tools/questions.py index`, `python tools/questions.py check`, table integrity checks and
  `git diff --check` passed. Cargo/clippy/ctest and game runs: n/a (documentation only).
- Only six docs files changed. No engine, tools, ABI, unreconciled rows, HANDOFF.md or README.md edits.
  Scratch evidence helpers remain uncommitted in build-runs/font-evidence-r2.py and font-x87-r2.py.
  q.py could not start Bash (E_ACCESSDENIED); native PowerShell used. Unicorn was unavailable; no dependency
  or tool was installed. Git printed a packed-refs.lock permission diagnostic while committing, but the
  detached HEAD commit completed and was verified; no shared-ref/config/maintenance action was attempted.

## Next steps
The implementer can ship the exact name map, emulate the native startup advance arithmetic and non-space
branch, and reconcile explicit surface-lock bookkeeping/returns using §7/§9. Native crash cases are documented
as such, with no invented false-return contract. No further RE question is required for FN15-FN17.
