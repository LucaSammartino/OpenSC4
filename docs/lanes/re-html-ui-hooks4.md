| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-html-ui-hooks4 — HTML hook and URL dispatch specification | Codex / re-html-ui-hooks4 | ready — docs publication; foreign domains explicitly partial | Coordinator may land; future foreign contracts are HU1–HU4 |

## Publication

[html-ui-hooks.md](../specs/html-ui-hooks.md) publishes the supported owner
contract for eight attributed hook entries (904 bytes) and DoURL (368 bytes),
9 entries / 1272 bytes total. Its per-body ledger separates supported routing,
state and return behavior from partial foreign or invalid-memory domains and
unknown original names. Two further constructor/destructor-shaped supporting
bodies were identified by vtable writes and the allocation caller (39 and 57
bytes); they are outside the nine-entry attribution total and earn no added
coverage claim. No whole-owner, implementation, native conformance or default
promotion is implied by publication.

The contract establishes both pointer views (B and B+8), exact interface IDs,
shared reference operations, Init replacement order, Shutdown clear-before-release,
and destruction without prior clearing. TestURL compares the scheme to `sc4`
case-insensitively; LoadDocument independently requires exact `sc4://` bytes.
Resource selection uses type `856DDBAC`, mnemonic or explicit group selection,
and an instance conversion only at length at least eight. The group store at
`0078D44E` was checked with its outstanding cdecl argument accounted for.

DoURL searches for lowercase `action`/`advice` at offset six without validating
the first six bytes. Action processing returns false without command dispatch;
advice forwards DoLink's AL. The resource reader ignores the second read's result
and returns true after an accepted size and resize. View3D's documented result
inversion remains correct. All five direct DoURL caller argument sequences were
checked, each passing zero as the unused second argument; capped consumer owners
were not reanalyzed.

## Evidence and admission

- Copied coordinator admission and host proof: `build-runs/evidence-run4/`.
  Full host admission passed, no prior logical family, FIRST round 1. Own current
  manifest: main `build-runs/run4-re-html-ui-hooks4-r1-manifest.json`, kind RE,
  source base `ec52575cee183bf015a243ef4fc9781ff9673b7f`. Detached HEAD was reused;
  no branch/ref switches or global host preflight retries.
- Original version: 1.1.641.0. SHA-256
  `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`
  matches the Windows export summary, image base `00400000`.
- Primary evidence: export byte-bearing instructions, original read-only PE
  literals/vtable words, and narrow primary caller sequences. Local decompilation
  was supporting navigation; native instructions resolved its wrong argument and
  stack-offset impressions. All evidence citations are in the spec.
- Canonical re_query/export access worked. Live REA was neither needed nor queried;
  no claim of live target readiness. A scratch helper's optional `pefile` import
  failed; read-only PE section mapping was then done with the standard library.
- Commits: `cf73f83c` admission/handoff; `75086033` substantive owner publication.
  This report accompanies final documentation validation and question indexing.

## Validation and limits

`python tools/questions.py index` regenerated the index; `python tools/questions.py
check` passed. Local documentation validation passed for three UTF-8 documents,
seven relative links, presence of all nine attributed addresses, and absence of
code fences in the spec. `git diff --check` passed. All vectors are proposed
instruction-derived expectations, with explicit foreign preconditions; none
were executed against native objects. The proposed examples cover literal
accept/reject/case/empty/prefix/resource/path/link/failure/output/reference order.

Cargo, clippy and ctest: n/a (documentation-only lane). Game labels: none;
shadow comparison: n/a. No builds, tests of implementation behavior, game/probes,
network actions, native calls, scripts/commands/links, browser/HTTP activity,
engine/tests reads, ABI table edits or version changes. No C ABI export changed;
no ABI proposal file was necessary. No shared coverage/map/default changes.

Git commits succeeded on detached HEAD and returned hashes despite an auxiliary
`packed-refs.lock` permission diagnostic; no attempt was made to change shared
refs/config or bypass hooks.

## Questions and next steps

Filed [HU1–HU4](../specs/questions/html-ui-hooks.md): foreign URL/encoding/hex
domains; service identity and read semantics; allocation/exception/unchecked
pointer domains; registration ownership and reentrancy. No preexisting question
was answered or moved. Index reports HU5 as next available in this file.

Ready for the coordinator's documentation merge. Implementers can use the
supported owner decisions and conditional traces; any future foreign-domain
research needs its own authorization and scope. No automatic continuation,
new family, native probe or further round is requested.
