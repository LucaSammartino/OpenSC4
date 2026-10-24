| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-nettool-nt30: retained corners and subclass replay fixtures | Codex / `re-nettool-nt30` (reserved detached worktree) | **ready**: NT30/NT31 answered; spec milestone `3378f8b09`; doccheck/check+links and questions check pass; no ABI change or game run | Replay controlled vectors, then wire subclass resource/recording hosts; retain six city differences pending fresh comparison. |

## Report

Extended [network-tools](../specs/network-tools.md) with §6.5, §8.1.2, §8.3 and conformance vectors 31–37.
Moved NT30 and NT31, preserving their question text, to the answered archive and regenerated the question index.
No implementation, ABI table, exported C ABI, coverage map or game asset changed.

### NT30: refresh and price inputs

The position pass refreshes all smoothing-set cell corners, including already solved entries; its height lookup
falls back to current terrain. The subsequent terrain-neighbour pass admits only placed unsolved incident
cells and leaves a corner unchanged when the network satisfier has no height. It uses neither the first pass's
terrain fallback nor its connection-node vertex offset. Solved indices suppress repeated neighbour admission.

Controlled fixtures explicitly specify all entry coordinates, corner ids and heights for the four reported
footprints and repeat: 11/17/9/11 entries, 44/68/36/44 corner observations and costs 110/170/90/55; repeat 110.
A stale-corner fixture changes 114 to 110 after refresh without adding entries. An admitted non-run neighbour
with a missing network height adds four earthwork units at factor 1 and is appended only once.

These fixtures are source-derived controlled inputs, **not native Konradshohe internal snapshots**. Public
totals do not identify unique native corners, neighbour inventory or solve inputs. Four observed city differences
110/113, 170/192, 90/98 and 55/61 remain; no fitted adjustment or coverage credit is proposed.

### NT31: subclass input contracts

Documented shipped Utilities tuning (uint32 spacing 10, float32 clearance 7, uint32 pole price 15, float32 wire
price 2), direction-mask pole mapping/resource requirements and recording-host operations. The controlled
13-step vector explicitly sets the pivot prefix count to 13, yielding three poles, ten wire cells and price 65.
Pricing alone certifies no pole/wire construction. Verified x87 multiplication has no float32 store before
truncation, qualifying the older network-geometry r24 shorthand; a large-count vector detects that distinction.

Specified all nine shipped road bridge identities in loader order, their minimum spans and selection inputs,
plus the model/height/occupant/terrain inventory needed by a recording host. Separate selection and retained
price vectors choose index 1 for an exact 16-step fit and price a 2..11 bridge record at 900. Neither is asserted
to recover the native city span, model or total 974. The generic-adapter 65/0 and 974/0 differences stay visible,
record-only and default-off; no whole-owner or pole/wire/model coverage is claimed.

### Evidence and checks

- Queried the main export with re_query first; used bounded decomp reads and byte-bearing instruction queries.
  No REA call was made: the brief says its session has no binary open. No REA failure/fallback occurred.
- Checked main glue-nettool-shadow5 before RE. That file has no Owner closure heading or `open:` rows;
  its six remaining probes and owner limits identify the blockers used here.
- `python tools/doccheck.py check`: PASS; `python tools/doccheck.py links`: PASS.
- `python tools/questions.py check`: PASS after index regeneration; `git diff --check`: PASS after removing
  an extra trailing blank line found by the first whitespace check.
- Ignored helper `build-runs/check-nt30-fixtures.py`: PASS for entry/corner counts, vertex ids and price arithmetic.
  This is an arithmetic review of documentation vectors, not a test against the original game.
- Cargo/clippy/CTest: n/a (RE documentation only). New game runs: none; no requests outstanding.
  Prior round-2 a/b/c/d evidence is clean but only 8/14 exact; this lane does not change that verdict.
- Questions filed: none. NT30 MATCHES and NT31 DIFFERS archived under `questions/answered/network-tools.md`.
  No proposed ABI rows.

## Handoff

- Completed: initial handoff `48aa64b69`; substantive specs/archive/index `3378f8b09`; final report and whitespace cleanup follow.
- No remaining RE action in this lane; enqueue kind re, state ready, using actual detached HEAD.
- Implementation next: run controlled refresh fixtures, provide subclass resources/recording hosts, and compare replay inputs before another city run.
- Native city corner, pole and bridge-record inventories were not captured; public totals are not substitutes.
- Preserve four cost differences and two subclass differences; keep shadow default-off and claim no new exactness.
- All edits are documentation. Worktree reused; main only read; no shared branch/ref changes, merge or push requested.
