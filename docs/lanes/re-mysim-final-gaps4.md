# re-mysim-final-gaps4 — answer MS13/MS14/MS15 (mysims §4.6/§4.7/§4.7.1/§4.12/§7.2)

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysim-final-gaps4 (MS13-MS15 location/import/commute contracts) | `re-mysim-final-gaps4` / `.claude/worktrees/re-mysim-final-gaps4` | ready; docs only; MS13-MS15 answered; MS16 open; questions/check and diff/check PASS; ABI unchanged; game n/a | 1. Prove factory defaults in MS16. 2. Implement the published contracts and proposed cases 21-25. |

## Report (2026-10-22)

Audited the preserved 4c714151 WIP before using it. The MySim spec delta against current main
1f636e4d is confined to MS13–MS15 sections and their conformance cases. No engine was read or changed.
The inherited modified PE reader was audited, saved to ignored `build-runs/rd-audited-round2.py`, and
restored to its tracked version; no helper code or extracted native output is included in the commits.

Commits: dfc58173 verifies location/import edges; 5040a329 publishes commute ordering and answers.
The final report commit also records the missing-service Lua-stack restoration exception in §4.6.

- **MS13, §4.7.1:** byte-verified magic-number world-to-cell conversion, NaN/large-coordinate examples,
  rounded world writes in SetCellPosition, and direct single-cell fallback after an occupant's lot miss.
  Proof: 00431370–004313FD, 004311B0–004311D7, 00431880–004318F4. Corrected WIP's claim that all int-to-float
  stores were exact and clarified that forced trip sampling changes a local copy.
- **MS14, §4.12:** count starts at zero per family (0043C27E with 16 pending argument bytes); signed `%d`
  key/face formats; short names keep their entire stem (0090F46D); missing gender/zodiac skips the member,
  whereas successfully read empty values continue. Unsigned all-ones count can wrap the loop indefinitely.
- **MS15, §§4.6/4.7/7.2:** CheckCommute's full caller order, independent configuration, occupant-bound
  rectangles, MySim neighbour cell, result exactly 1, slot-26 display-scaled getter, low32 conversion without
  clamping, and failure/replacement-owner callbacks before Release. Proof: 0042D9E0–0042DB40,
  0042C8E0–0042C930, 006D6030–006D603F, 009EEF04–009EEF78. Trip sampling uses local world position for
  congestion and local cells for ratio, both forced to zero; active means state nonzero, and subjects use
  the new trip-state argument with retention outside states 2/3. Missing service exits skip Lua restoration.

**Validation:** questions index regenerated; `python tools/questions.py check` PASS; `git diff --check`
PASS. MySim questions: open 1, answered 15, next MS17. Added five proposed conformance case groups
(§12 cases 21–25), with explicit branch/order discriminators and representable numeric operands.
Arithmetic/formatting examples are instruction-derived, not native runtime measurements. No invented
host commute-time values or reused RNG draws, no conformance code, no cargo/clippy/ctest or game run.
Shadow verdict n/a. ABI and exports unchanged; no ABI proposal file.

**Questions:** moved original MS13–MS15 entries to `docs/specs/questions/answered/mysims.md`, each marked
ANSWERED 2026-10-22 (re-mysim-final-gaps4), DIFFERS with MATCHES subparts. Filed **MS16** in
`docs/specs/questions/mysims.md` for inherited pathfinder factory configuration; no numeric defaults asserted.
The factory 0070D860 lacks static decode/decomp, the raw-disassembly fallback lacks capstone, and REA
returned `target_unavailable: No app is open`. Continued with static export/decomp proof for all answered
caller contracts. q.py also could not start Bash; capped PowerShell/local Python worked.

**Next:** prove MS16 from factory/Init bytes when available, then implement and verify the published
contracts and proposed vectors. Generated QUESTIONS.md reflects this worktree's question set; the merge
train must regenerate it against its combined tree. Detached HEAD is intentional; no refs, merge or push.
