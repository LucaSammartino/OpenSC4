| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-pr-fonts5 (PR 10 font spec harvest) | Codex / detached worktree `re-pr-fonts5` | ready: harvested a12daabf as 7cf52fd7 with amarret's authorship; main preserved; doccheck and questions pass | Land docs and RE tables; FN23 remains open |

## Report

Harvested local PR 10 commit `a12daabf159711392be0c3ea2edca232337b4be9`, cherry-pick provenance retained in
`7cf52fd7`, author **amarret <51444322+amarret@users.noreply.github.com>**. No fetch, GitHub access, branch change,
or shared-ref update. Base was `4eebc20b`. The reserved worktree remains detached for supervisor publication.

### Changes and conflict resolution

- Added drawing-fonts §§14–15: MXF wrapper and XOR contract; bounded summary of the separate glyph-cache holder.
  Added outline entries and aligned the introductory scope with the summary.
- Retained the PR's UIW-10, V3D-9 and INVENTORY-M6 §4.2 cross-references: MXF is specified; UI grammar remains open.
- QUESTIONS.md was the only cherry-pick conflict. Regenerated with `python tools/questions.py index` rather than
  merging it. Six font questions remain open, 17 answered; next ID FN24.
- No INVENTORY-M6 generator or generated marker was found (searched tools/ and .claude/ scripts); kept its single
  handwritten cross-reference update. No other inventory content changed.
- RE TSV union uses the address key. Every main names_windows row remains verbatim, including the 43 duplicate
  keys the PR would rename; only 3 new addresses were added. The new fonts.tsv holds all 46 unique symbol rows.
  The imported `docs/lanes/re-fonts.md` is historical contributor evidence: its 43-renames claim and +134-byte/
  +2-function local-export measurement describe the contributor's run, not this lane's measurements.

### Clean-room review and corrections

Reviewed all harvested spec prose under CONTRIBUTING.md's Specs rule. No original code, assembly or decompiler
local variables appear in the resulting specs. Specific changes:

1. Replaced §14's opener/member-offset walkthrough (`+0x80/+0x84/+0x88`, byte-swap and callback sequence) with
   observable recognition, logical offsets, read transformation and success/failure contracts. Short citations remain.
2. Corrected the harvested requirement that a writer must obfuscate the whole font. The reader supports L = 0 and
   a partially obfuscated payload. Shipped L/key values are a writer choice; added worked XOR-boundary examples.
3. Removed §15's unsupported categorical assertion that every hardware target draws through its own renderer.
   Retained the observed interface test and explicitly deferred the full hardware/fallback contract to FN23.
4. Clarified that the unclaimed coverage is the larger holder's, without reopening the separate cGZFont owner.

Instruction checks used the main checkout's read-only export through re_query.py: `00896D70–00896DD1`
(recognition, big-endian L/key), `00896DDD–00896E47` (read result and unsigned XOR interval), `00896E4A–00896E62`
(Boolean callback), and `0082A2F0` cache placement/pixel paths. Existing decomp material was consulted only for
the holder summary. No REA call was needed. Shipped-file counts, types and padding observations are inherited
contributor evidence, not newly measured from game assets in this lane.

### Owner closure comparison and questions

Compared only the specifically requested `engine/core/src/fonts/README.md` Owner closure table, without reading
engine source. **No real contradiction found.** Its implemented cGZFont sections 1–13 and explicit FN18–FN22
residuals are preserved. New §14 is the file-stream contract; §15 is class `0x8B3255F9`, separate from cGZFont
and the small default renderer. The harvested no-coverage statement concerns that holder, consistent with FR3.

Harvested **FN23** in `docs/specs/questions/drawing-fonts.md` (complete cache-holder and hardware contract).
No additional contradiction question was warranted; no question was answered or archived in this lane.

### Validation and landing

- `python tools/doccheck.py check`: **PASS**.
- `python tools/questions.py check`: **PASS** after index regeneration.
- `python build-runs/audit-re-pr-fonts5.py`: **PASS**. Main's drawing-fonts §§2–12 are text-identical, preserving
  FN11–FN17 rules and all worked vectors; FN18–FN22 remain an unchanged question-file prefix. Every main RE row
  survives verbatim. Answered archives, engine/, external/ and tools/coverage-map.tsv are untouched.
- Ordinary `git diff --check` flags the 46 intentional empty final `mac_x86` fields in the imported TSV as trailing
  tabs. Retained the four-column TSV format; whitespace check excluding blank-at-EOL diagnostics passes.
- Cargo, clippy, ctest, conformance and game runs: n/a for this documentation/RE-table harvest. No ABI export change.
- Next: coordinator lands this lane; future RE can resolve FN23 and FR3. No merge, push or coverage-map change here.
