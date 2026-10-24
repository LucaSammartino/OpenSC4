| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-nettool-nt27: scripted result contracts NT27–NT29 | `Codex` / `re-nettool-nt27` (RE, detached reserved worktree) | **ready**: NT27–NT29 answered; doccheck and questions PASS; ABI exports unchanged; no new game runs | Implement corrected footprint/failure/cost contracts; wire subclass adapters and rerun shadow |

## Report

`5aa06e32e` extends network-tools §§2.1, 3.5, 5.3, 8.1.1 and 14 and adds conformance
vectors 20–30. `55289c82a` preserves and answers NT27–NT29 in
`docs/specs/questions/answered/network-tools.md` and regenerates `docs/specs/QUESTIONS.md`.
The questions existed on `glue-nettool-shadow5` but were absent from this base's open file;
their original text/provenance was recovered from that branch. No other open questions were closed.

NT27: slot 23 replaces a caller-owned coordinate vector with a copy. Slots 25/26/27
return current valid/straight/total steps before another draw, Reset or Shutdown.
There is no public solved-record getter: coordinates cannot certify pieces, types or
orientations. Missing bridge and power-line adapters remain implementation work.

NT28: staircase turn cells belong to ordinary dragged rows. A diagonal (20,4)–(28,12)
has 17 cells in 9 steps; (14,4)–(18,8) has 9 in 5. Minor coordinates increase within
each row even when the drag reverses. An unaligned horizontal avenue at z=28 uses
z=27 then z=28 for every x row. Slot 14 normalizes any nonzero internal failure
reason to EAX=1 (`0x0062C8F0`–`0x0062C8FD`); there is no new literal-1 reason.
The Boolean logs cannot identify the particular rejecting pass. Nonempty refused
runs retain their drag footprint but return false with the default stop-on-failure.

NT29: price the retained solved entries and their final cached network corners
against live terrain at stored vertex IDs. Commit can append solved neighbours;
the public drag is not the universal cost inventory. Shared vertices are charged
once per corner occurrence, with float32 accumulation after each addition. Cell
`+0x52` is an intersection marker, not an already-built marker; placed `+0x51`
does not suppress normal cell cost. This explains why repeat construction can
still cost 110. Flat 17-cell/9-cell road footprints cost 170/90 with no other charges.
The observed 3/6 residual cost differences cannot reveal individual height inputs;
the spec gives exact input/lifetime rules, with no guessed adjustment or tolerance.

Evidence: main checkout's read-only `tools/re_query.py`, Windows export and the
corresponding decomp functions; numeric details checked against instruction bytes.
No REA call was made because no binary is open in that session. No engine, plugin,
capi, coverage map or docs/abi files were read/edited as implementation material.
The shadow5 branch report has no Owner closure table or `open:` rows; its three
question entries and per-probe results supplied the closure requests.

Proposed ABI-note corrections: `build-runs/abi-rows-re-nettool-nt27.tsv`, five
replacement rows in interfaces.tsv format (14/23/25/26/27). No C ABI export changed.
The coordinator must apply those notes separately; the TSV is ignored lane evidence.

Validation: `python tools/questions.py index`, `python tools/questions.py check`,
`python tools/doccheck.py check`, and `git diff --check` PASS. Cargo/clippy/CTest
are n/a for this spec-only change. Git commits succeed despite a nonfatal denied
`packed-refs.lock` attempt; no shared refs/config/hooks were changed.

No new game requests or runs. Existing corrected shadow5 runs
`nettool-shadow5-r1-20261024-b` and `nettool-shadow5-r1-20261024-c` remain **5/14 exact**
through 2032, turbo and normal respectively, as reported by that lane. These answers
do not claim a corrected replay, new exactness, or whole-owner coverage.

## Handoff

- Complete: NT27–NT29 answered and archived; spec and question checks pass.
- Next: implement the documented contracts and subclass/resource adapters, then rerun all 14 probes.
- Preserve measured shadow differences until corrected runs establish exactness; no game request is outstanding here.
