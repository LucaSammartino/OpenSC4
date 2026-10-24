| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-budget-window-bw22: subbudget owner contracts | Codex; `re-budget-window-bw22` (detached worktree) | ready; doccheck passed; partial BW17 closure | Implement BW21 observation boundary and ordinance rules; follow up BW22–BW25. |

## Report

Extended budget-window §1.1 and §9 with the byte-verified constructor ABI, global heap allocation/free pair,
deleting-destructor flags, nonmodal population boundary, field/control observations and four lifecycle vectors.
The constructor takes only ECX=this and returns the same pointer in EAX. Native population without a modal
loop still requires working generated windows and child controls; no window-free owner calculation API or
row-result getter was found in the six-slot primary vtable. The shared +0x90/+0x94 accumulator cannot expose
both totals after both category passes. No native off-screen adapter or exact shadow comparison was run.

Extended budget-window-bodies §2 with the zero/nonzero period scale, B's signed negation, ordinance refresh
provider calls, unchanged edit pairs, positional reorder quirk, viewport/map rules, whole-category totals and
seven discriminating vectors (including wraparound). Corrected GetBudgetSegment's inventory description:
it creates a resource-backed bitmap segment, rather than selecting budget data.

BW21 is archived as answered (the supplied question was absent from both question files at this base).
BW17 is archived explicitly as **partial closure**, preserving the whole-owner limitation. Existing evidence
already covers ordinance/neighbor enumeration, restore-record acquisition/references, balance-sheet and
loan bodies; this round adds ordinance refresh/scale/formatting and splits the unproved remainder:
BW22 neighbour options/refresh/metadata/references; BW23 individual service arithmetic/saved floats and
initializer continuation; BW24 group refresh remainder; BW25 setup/property/popup/widget ownership.
BW18–BW20 and runtime BW11 remain open. Regenerated docs/specs/QUESTIONS.md (8 open budget-window entries).

Validation: **doccheck check passed**, log `build-runs/doccheck-bw22.log`; `git diff --check` passed.
The first q.py gate attempt failed before doccheck launch because Git Bash CreateFileMapping returned
Win32 error 5; a direct foreground Python subprocess captured the successful gate separately.
Cargo/clippy/ctest: n/a (spec-only). Game runs: none. ABI exports/proposed ABI rows: none.
Evidence uses tools/re_query.py and main-checkout decomp; numeric/ABI claims checked against cited
instructions. REA was not queried because the supplied session has no binary open. No engine edits.

## Handoff

- Goal complete for this partial-closure round; no whole-owner completion claimed.
- Done: lane start 1866478f3; BW21 contracts/vectors 2257f2663; ordinance evidence and question split in the final spec commit.
- Next: coordinator may land the docs; schedule BW22–BW25 as bounded audits, keeping BW18–BW20 independent.
- Off-screen shadow adapter must create actual native controls; no runtime recipe verdict exists yet.
- Local evidence: build-runs/asm-*.txt and asm_slice.py; gate: build-runs/doccheck-bw22.log.
- Worktree is intentionally detached; supervisor publishes commits to the reserved lane branch.
