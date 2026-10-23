| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysim-ms5: dispatch behavior closure | Codex / re-mysim-ms5 | ready: MS19 answered; MS18 partial | Implement MS19 and verified MS18 cases; further RE required for complete owner closure |

## Report

Published behavior only in `docs/specs/mysims.md`; no engine, plugin, C API,
ABI table or coverage-map changes. The requested complete MS18/MS19 closure
was **not achieved**: MS19 is answered and archived, while MS18 remains open
with an explicit narrower residual. This is a ready documentation contribution,
not a claim that the dispatch owner can now be closed.

### Changes and evidence

- `99b884ae`: MS19 equality in §4.7.1 and named teleport in §7.2.1,
  with five normative fixtures in §12.27. Equality requires both flag bytes,
  occupant identity, cell and region coordinates; world/timing do not participate.
  Teleport unconditionally copies commute to current, recomputes deadline,
  sets travel FE/state 3/flag 2, and notifies flags before state without arrival.
- `993d6199`: partial MS18 contracts in §4.6.2, corrected tuning explanation
  in §4.11, and five fixtures in §12.28. The proximity helper measures current
  rectangle corners against the destination rectangle, rounds distances to
  float32 and truncates the minimum linear distance before comparison with
  squared tuning. Strict idle and inclusive final-marker boundaries differ.
  Other settled details: ordinary switch wait classes; first-match transit
  stop selection, purpose/group table, timestamp and EBA0732E ordering;
  strict transit timeout; final-marker notices; common notification tail.
- Queried the local Windows export first, then inspected bounded sections
  of `decomp/cSC4MySimDispatch.c`, `cSC4MySim.c` and
  `cSC4MySimInputControlDispatch.c` in the main checkout. Numeric contracts
  were checked against instruction bytes and cited in the owning spec.
  Important citations: equality 00431640/00424EC0; teleport 00431C70;
  rectangle/distance 00431880/00431900 and sqrt 005D7324;
  boundary comparisons 00435B94 and 004361FF; stop/timeout 00435EF4–00435FB9.
- No REA request was needed or made. Export/decompiler symbol/signature errors
  were treated as evidence limitations, not copied into behavior claims.
  The investigation ended at measured context 95,883 / 150,000, within the
  requested two-thirds investigation allowance; publication and verification
  followed. The unresolved scope was retained rather than guessed.

### Verification

- `python tools/doccheck.py --ref main check`: **PASS**, `doccheck check: ok`.
- `python tools/questions.py index`: **PASS**; MS19 removed from open index,
  MS18 retained. Ten new normative specification fixtures; none represented
  as executable tests or native traces.
- Cargo, clippy and CTest: **n/a**, documentation-only RE lane.
- Game runs: **none**. No shadow comparison or replacement claim; no request
  outstanding. C ABI exports unchanged; no proposed ABI rows.
- Initial bare `doccheck.py` invocation failed for a missing subcommand;
  corrected invocation above passed. Initial q.py failed to start Git Bash
  (CreateFileMapping error 5); subsequent shell work used capped PowerShell
  and direct Python. Git printed a packed-refs lock permission diagnostic
  during detached commits but completed each commit successfully; no shared
  refs/config were changed to work around it.

### Questions and next steps

MS19 moved intact to `docs/specs/questions/answered/mysims.md`, marked
`ANSWERED 2026-10-23 (re-mysim-ms5): DIFFERS`, with its rule in the spec.
MS18 remains in `docs/specs/questions/mysims.md`: complete branch/mask matrix,
idle/FE/FF/cancellation behavior, neighbour retry index/enumeration, all
EBA06BD0/completion/recovery notice order, dispatch-tool/pylon input lifecycle,
mission callbacks and delegated transport/model RNG still require investigation.
Do not remove all UnsupportedMs18 branches on the strength of this report.

The engine lane can implement MS19 and the verified MS18 fixtures independently.
A successor RE lane must finish the residual before dispatch owner closure.

## Handoff

- Ready contribution: MS19 complete, MS18 partial; full report above.
- Commits: 99b884ae (MS19), 993d6199 (MS18 verified subset).
- Next RE: finish the residual MS18 entry; do not re-open or re-investigate MS19.
- Resume evidence at switchTravel 00435840, updateState 004365C0,
  basicDispatch 004352B0, neighbour helper 00434C40; input owner 00439380–00439E50.
- Input/mission pseudocode has bad signatures; resolve callback arguments from instructions.
- No game runs, ABI rows, or external service request pending.
- Worktree is detached by supervisor instruction; queue uses actual HEAD.
