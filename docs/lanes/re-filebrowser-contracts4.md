| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-filebrowser-contracts4: browser ownership and remaining contracts | Codex / `re-filebrowser-contracts4` (RE spec lane, detached worktree) | ready for docs merge; owner incomplete and parked after final third round; a3dbc38c / 8bc45fdc; questions check and whitespace PASS; cargo/clippy/ctest/game n/a; ABI unchanged; FBQ3 answered, FBQ1/2/4/5/6 parked | 1. Coordinator publishes detached commits. 2. Preserve exact residual ledger; no fourth round or complete-owner credit. |

## Published contracts

`a3dbc38c` publishes lifecycle and ownership before call 30. `8bc45fdc`
publishes control/property, comparison and fitting contracts and the final
inventory audit. Only browser spec/question files, generated questions index
and this report changed; no engine, tools, ABI tables or game files changed.

- Recovered constructor `009A57C4` (710 B), absent from the preserved union;
  defaults, descriptor/string ownership and actual Init size 396 × 236.
- Distinguished secondary tables `00A8D000` and constructor-installed
  `00ADCF30`, their two destructor thunks/wrappers and actual destructor.
- Proved Shutdown's 29 retained-reference fields and omission of primary
  `+0x280`; file/drive/custom-entry clear methods; Init setup order and failure
  preconditions. Direct destruction does not call browser Shutdown; normal
  final Release does, through shared FinalRelease `009CFA1D`, before deletion.
- Specified EnableChildControl's byte storage and detach/release behavior,
  including disable exceptions for IDs `0xBAA55D51` and `0xBAA55D55` and no
  control recreation when enabling. Full AddControls/layout remains partial.
- Specified selected primary accessors, return widths, borrowed window
  pointer, flag tests, dirty setters, exact string-copy getters and pending
  name copy-plus-append (no separator, no immediate selection).
- Specified default comparator's type grouping, lowercase byte ordering,
  primary/secondary key rules and duplicate-parent exception; specified
  fitting's strict marker-width inequality, refit/trim/append and null-font path.

## Measured inventory and limits

Section A retains all **174 distinct addresses / 28,907 export bytes** exactly
once, with 107 primary slots / 106 distinct targets. Section A.1 reconciles
100 primary browser bodies / 15,344 B; 13 non-primary browser method/callback
bodies / 9,404 B; five folded targets / 54 B; six thunks / 45 B; two deleting
wrappers / 58 B; four record helpers / 118 B; two shared string helpers / 61 B;
and 42 unresolved attribution candidates / 3,823 B.

Within the original union, **60 entries / 7,029 B** have complete contracts,
including shared targets/wrappers/helpers. These are not 60 unique browser
methods. Attributed browser bodies alone: **41 / 6,693 B** specified;
**72 / 18,055 B** still partial or unresolved. Adding the newly recovered
constructor gives 42 specified browser method/callback bodies / 7,403 B;
including specified wrappers/helpers gives 61 entries / 7,739 B. The augmented
inventory is 175 / 29,617 B. Its exact total-owned-method denominator remains
unproved because helper attribution is incomplete. No complete-owner credit.

All **114 original residual entries / 21,878 B** are individually listed with
addresses, sizes and question destinations in the spec. Residual totals:

| Question | Entries | Bytes |
|---|---:|---:|
| FBQ1 procedural pixel/failure recipes | 4 | 6,055 |
| FBQ2 controls/messages/paint | 3 | 4,009 |
| FBQ4 remaining primary properties/mutations | 62 | 3,835 |
| FBQ5 population/column sizing/selection restore | 3 | 4,156 |
| FBQ6 helper attribution | 42 | 3,823 |

The largest residuals remain `0099EF11` 3,423 B, `009A1F77` 3,149 B,
`009A4EBA` 2,241 B, `009A1163` 1,938 B, `009A473F` 1,007 B,
`009A4B2E` 908 B, `009A0ACC` 687 B and suspicious comparator entry
`005C3A63` 575 B. This final logical third round is parked; no restart.

## Questions and verification

FBQ3 moved to `docs/specs/questions/answered/win-filebrowser.md`, marked
ANSWERED 2026-10-22 (re-filebrowser-contracts4): DIFFERS. Its rules are in
sections 1.1 and 8.5–8.6. FBQ1/2/4/5/6 remain in
`docs/specs/questions/win-filebrowser.md`, narrowed to exact residual contracts.
No unrelated widget owner was explored or modified.

Export queries preceded decomp reads; numeric widths, counts, comparisons and
arithmetic were verified against native instructions with addresses cited.
Native executable data confirmed both vtables, EnableChildControl jump cases
and default strings. Export/local evidence sufficed; REA was not invoked and
there was no REA fallback/error. Scratch audit scripts/logs stay ignored under
this worktree's `build-runs/`; they are not specification source or committed code.

Validation: questions index regenerated; `tools/questions.py check` PASS;
`git diff --check` PASS; inventory audit PASS (174 unique / 28,907 B;
107 slots; disjoint residual totals). Cargo/clippy/ctest and game runs n/a
for this docs-only publication. ABI exports unchanged; no proposed ABI rows.
Detached commits succeeded despite nonfatal Git packed-refs maintenance-lock
permission diagnostics; no refs/config/maintenance permissions were changed.
