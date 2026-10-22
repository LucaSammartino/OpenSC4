| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span-context-gaps4 (span-render-4444 SP1 packed blend, SP6 source setup and return addresses) | Codex / `re-span-context-gaps4` (spec lane) | blocked: docs complete but uncommitted; Git index.lock ACL denied; questions/static checks pass; no ABI or new game verdict | 1. Restore Git metadata write access. 2. Commit the five changed docs. 3. Queue committed tip as docs/ready. |

## Report (2026-10-22)

The documentation task is complete in the working tree, but this lane is **not ready to merge** because Git
cannot write its metadata. Tip remains the inherited WIP commit `c631ec01`; none of this round's edits is committed.

### Changes

- Audited inherited commits `1806028e` and `c631ec01`: they only created/updated the lane note. No partial spec or
  implementation changes were preserved. Ignored helpers were checked before use. At audit time the worktree's
  span spec and open questions equalled current main; no merge or broad discovery was performed.
- `docs/specs/span-render-4444.md` section 6.1 resolves SP1. Alpha is unchecked, with the exact wrapped dword map
  giving weights 0..31. Bytes `0x0084801E`–`0x0084806C` prove a shared 32-bit red/blue intermediate, logical shift
  and final field masks. Separate-channel low-nibble arithmetic differs above weight 16. Added 15 discriminating
  weight vectors, seven boundary/carry vectors and two two-pixel ramp checks. Example: source `0000`, destination
  `880F`, raw alpha `01000000` gives `FF0F`, not separate-channel `F00F`.
- Section 1.3.1 resolves SP6a from setters `0x0082FFD0`/`0x0082FE70` and consumer `0x008302E0`: x/y are destination
  placement, sx/sy forward magnification, stored steps their quantized reciprocals. SetBltSource writes a
  destination edge rectangle; SetBltTiling reads a source texel edge rectangle. Added six nonorigin/scale/rounding
  cases, a nonorigin tile and failure/no-write expectations. This agrees with shared span-render-1555.md section
  6.1, whose applicability is proved by the same context functions and Init's 4444 vtable selection. Current main's
  draw-context.md describes SC4DrawContext (3D), so its contracts were not transferred to this 2D context.
- Section 1.3.2 resolves SP6b: all seven sites are three-byte CALL instructions; exact returns are site+3, with a
  CALL/return/slot/4444-target table and classifier expectations. The fixture's old site+1..7 window is too broad.
- Moved SP1 and SP6, preserving their provenance and previous measured game evidence, to
  `docs/specs/questions/answered/span-render-4444.md`, both marked **ANSWERED 2026-10-22 (re-span-context-gaps4): DIFFERS**.
  Regenerated `docs/specs/QUESTIONS.md`: span4444 now has two open, four answered. SP2 and SP4 remain open.

### Validation and limits

- `python tools/questions.py check`: PASS. `git diff --check`: PASS.
- `python build-runs/audit-span-context-gaps4.py`: PASS, checking all seven CALL/return/target pairs against the
  executable bytes, three buffer/context slots, alpha shift/store widths, 15 weight vectors, seven alpha
  boundaries, two ramps, six placement cases, tile samples and nine rounding ties. This is an ignored local
  arithmetic/byte audit, not a committed tool or game run.
- Audited `build-runs/sp1vec.py` arithmetic sweep: 2,097,152 R/B combinations across weights 0..31, fixed G=7/dG=5;
  25,152 differ from separate channels, all above weight 16. This checks formulas derived from the bytes, not
  current core behaviour. No engine files were read or edited.
- Export and instruction bytes sufficed; no REA query was needed. No new game runs, no new shipped-hit claim,
  no shadow exactness/replacement credit, no ABI exports/rows/version changes. Cargo/clippy/ctest: n/a (docs only).

### Commit blocker and next steps

`git add` and `git commit -F` failed with `Unable to create .../.git/worktrees/re-span-context-gaps4/index.lock:
Permission denied`. The metadata directory ACL contains explicit write-deny entries. The quiet Bash wrapper
also failed with `Bash/Service/CreateInstance/E_ACCESSDENIED`; direct PowerShell reads/edits and checks work.
No ACL or sandbox settings were changed, and no merge-queue file was written for an uncommitted result.

After the coordinator restores Git metadata write access, stage only these five files: this lane report,
`docs/specs/span-render-4444.md`, `docs/specs/questions/span-render-4444.md`,
`docs/specs/questions/answered/span-render-4444.md`, `docs/specs/QUESTIONS.md`. Commit with
`git -C <WT> commit -F <WT>/build-runs/msg-re-span-context-gaps4-2.txt` (UTF-8 without BOM, Co-Authored-By footer).
Then update this report with the real commit hash and ready state, commit that, and write
`<WT>/build-runs/merge-queue/re-span-context-gaps4.json` using docs/merge-train.md: lane/branch
`re-span-context-gaps4`, kind `docs`, abi false, game_runs [], the committed full tip, state ready. Never queue
the inherited `c631ec01` as though it contained this work. No further RE is needed for SP1/SP6.


### Coordinator delivery recovery (2026-10-22)
After the worker stopped on sandbox-denied Git writes, the coordinator reviewed and committed these exact docs files. The original guard result and proof evidence remain preserved; no extra lane round or runtime result is claimed. The queue records the real committed tip.
