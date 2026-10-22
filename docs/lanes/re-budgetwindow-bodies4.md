| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-budgetwindow-bodies4 | Codex / `re-budgetwindow-bodies4` | ready: bounded partial BW17 spec; owner incomplete | Review five proposed ABI rows; remaining body questions stay open. |

## Report

Published a bounded partial response to BW17 in `docs/specs/budget-window-bodies.md`, linked from the
main budget-window spec. This is spec text only. Neither native owner is complete, and none of its
unproved methods may be replaced by a computed-result host callback and counted as implemented.

### Changes and evidence

- Preserved milestones: `c196156e` exported owner/slot inventory and ordinance population;
  `94730e06` neighbour scans/counts/selection and CalculateValues arithmetic.
- `69b3942c` reviews and corrects the preserved arithmetic patch before further body work. The combo
  special branch computes k=trunc(A/M)-trunc(A/I)+S and low32(M*k), resets selection and uses common
  LTEXT `0x2A5794A0`, then slot17(M); repeated public reads remain distinct. Byte evidence:
  `0078BE92`–`0078BFA4`. `UpdateBalanceSheetInfo` has no initial window null guard (`00767998`–`007679A5`).
- `281b2522` adds individual enumeration/restore acquisition and a bounded A-side group refresh proof.
  Individual restore records are 12 bytes, retaining one original occupant reference each; rebuild=true
  includes all returned occupants before viewport selection, while false preserves the original snapshot.
  Min(count,9) limits drawing; annual amount wraps at 32 bits before signed accumulation. Service-value
  computation is explicitly still open. Main proof ranges: `00785818`–`0078595A`, `00785DD5`–`00785E85`,
  `007860E0`–`0078612A`, copy/grow/release helpers `0077ACA0`/`0077D8C0`/`0077C480`.
- Group UpdateValues is the largest remaining named owner body (6265 bytes). Published its raw group and
  department provider calls, busy guard/cleanup, slider-vector clear, and A traversal/header reuse slice.
  Separate scroll and quota counters matter: skipped flagged items before the scroll offset still advance
  the quota; skipped headers do not. Worked example distinguishes a naive nine-row slice. B, A item
  rendering, slider rebuild and stale-row cleanup remain open. Evidence: `00789AC1`–`00789E63`,
  `0078A008`–`0078A01F`, `0078AFDC`–`0078B013`.

### ABI proposal

Exactly five proposed rows are committed in `build-runs/abi-rows-re-budgetwindow-bodies4.tsv`, in the
vtable TSV format: neighbour primary table `00AB1DD8`, slots 8, 10, 16, 17 and 22. Raw folded getter/setter
instructions were rechecked this round (`004ED900`, `00A6CAB0`, `0080BC00`, `006CE240`, `006CE110`).
Names from folded unrelated functions are not used as semantic evidence. The earlier ignored pending-abi
scratch is superseded by this canonical file. Coordinator review/application/gating is required; no
`docs/abi/` edits, C ABI export changes or ABI-version changes occurred.

### Validation and limits

`python tools/questions.py index` and `git diff --check` pass. BW17 remains in
`docs/specs/questions/budget-window.md` with explicit residual addresses; no entry moved to answered.
No cargo, clippy, ctest or game runs apply to this documentation-only change; no shadow comparison.
All reads of original export/decomp used the main checkout; no engine reads or edits occurred.

The previous round's REA request for `0077C660` failed `target_unavailable` (no app open); saved decomp
also failed there, so instruction export supplied its proof. This round used export first and saved decomp
only as navigation. The malformed `00786F62` fragment remains unreconciled; its bogus first decoded call
must not establish behaviour. Budget slot11/13 bodies remain undecoded as already noted in section 4.
No new tools were installed. Git commits succeeded on detached HEAD despite a nonfatal packed-refs.lock
permission warning; no refs, shared config or hooks were changed manually.

### Remaining work

Companion section 7 and the BW17 question retain three audit groups: ordinance/neighbour refresh/options;
individual service calculations plus full group B/slider/control reuse; setup/popup/failure references and
remaining panel loan/tax/formatting/rendering bodies. Read those exact paths before claiming full owner
coverage. The ready queue publishes this bounded evidence, not BW17 closure or runtime promotion.
