| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-au50 (audio owner contracts) | Codex / `re-audio-au50` (RE) | ready; AU45-AU49 and AA14 answered; questions index regenerated; doccheck passed; ABI unchanged | Coordinator lands reserved lane; core-audio-close11 consumes contracts |

## Report

Closed the six questions imported from core-audio-close11. Their bodies were read with git show;
they were absent from this worktree's open-question files. AU45 was already answered in debbb740c.
This round archives AU47 (21399ceb3), specifies AU46/AU49 (6fe5ae5b5) and specifies AU48/AA14
(f066f1a23). Answers retain the original question bodies in the corresponding answered files.

- audio.md §8.3.1: SoundManager defaults, lifetime, payload gates, reset/enumeration/volume/focus,
  stop-all, tick/ramp arithmetic, start failures, completion references and foreign position writes.
- audio.md §11.3: director registration, start, PostAppInit result policy and shutdown/destruction
  ordering, including failure without transactional rollback.
- audio.md §10.7.1: AEMS views/defaults, Init/Shutdown, backend results, callbacks, banks and
  file ownership, two unchecked GetSize calls, failed/short-read and tuning fallback behavior.
- audio-ambience.md §8.1.1/8.3: layer lifetime, static references, occupant/loop/freshness slots,
  failed-first-Init latch, repeated filter creation and process cache defaults.

Each new contract includes prose conformance cases. Literal oddities are specified rather than
left open: focus/stop-all DoEvent false after effects, singular loop removal without stop,
extra Release in singular freshness removal, zero-success bank registrar and file Close result
affecting resource-load success. Unsafe original allocation/size failures are documented as
failure boundaries, not instructions to corrupt replacement memory.

Evidence came from main-checkout re_query.py and bounded class decomp ranges. Numeric contracts
were checked against asm --bytes; PE constants were read with an ignored local build-runs helper.
REA was not used because this session has no binary open. The source lane's current lane file
had no literal `open:` closure rows; its six filed question bodies supplied the requested gaps.

Validation: questions.py index regenerated docs/specs/QUESTIONS.md; doccheck.py check passed
2026-10-24. Cargo/clippy/ctest and game runs are n/a for spec-only work. No new questions filed;
unrelated AU40-AU44 and AA10/AA12/AA13 remain untouched. No engine, plugin, capi, ABI-table,
tools/coverage-map.tsv or C ABI changes. No proposed ABI rows and no game requests.

## Handoff

- Ready for the supervisor to publish detached HEAD to reserved branch re-audio-au50 and land it.
- core-audio-close11 should consume the cited sections and implement the foreign stages.
- Queue uses actual full HEAD; no branch/ref updates, merge or push performed here.
