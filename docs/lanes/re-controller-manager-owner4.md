| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-controller-manager-owner4 (controller manager owner contract) | Codex / `re-controller-manager-owner4` | ready: bounded supplement; doccheck/questions/diff checks pass; no ABI/game run | Land documentation; AT21 residual dependencies remain open; full manager coverage unclaimed |

## Publication

Added [automata-controller-manager.md](../specs/automata-controller-manager.md), linked
from automata.md §3. First dedicated **bounded** owner supplement, not completion of
the canonical automata family. `4e3b6050` committed the substantive initialized teardown
contract; `a3c8eadf` added city/message/queue/tick boundaries, six literal reference
traces and the body ledger. No implementation, ABI, factory implementation, coverage
map or warning annotation changed. No proposed ABI rows.

## Native results

- Shutdown unregisters debug/tick/message targets, Releases script then clock, drains
  queue references, clears grid storage, drains controller references, clears its guard.
- Queue-map drain Releases the occupant key before queued automata, without automaton
  Shutdown. Controller-map drain calls Shutdown then Release per list occurrence,
  then Releases occupant keys. Lists stay linked during these callbacks; no deduplication.
- Final manager Release explicitly invokes virtual Shutdown before storage destruction.
  Completed repeated Shutdown is guarded; callback mutation/recursive teardown is unclosed.
- City controller-manager Shutdown `0045B2F2` precedes occupant-manager Shutdown
  `0045B833`. PreCityShutdown has no cleanup branch or subscription in this manager.
  Loaded PostCityInit resolution and received RemoveOccupant are specified with foreign
  replies as inputs. Group-null removal's callback/unlink order differs from name removal.
- Borrowed queue access, occupant AddRef, duplicate suppression, automaton AddRef/Release,
  predecessor repair, close drain, tick gate and controller callback admission are published.
  Numeric clock arithmetic and controller body semantics remain separate.

## Evidence and measurement

Working original version 1.1.641.0 SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`
matches export-summary.txt (base 00400000; 35,637 functions; 2,193,251 instructions).
Export queries came first, pseudocode supplied navigation, and byte-bearing instruction
queries established release slots, guards, receiver adjustments, arguments and ordering.
No live REA needed/attempted, no live-readiness claim. Own round-1 manifest's lane/kind/WT
verified and normal guard checked; no target/server/process manipulation or second round.

Exact ledger: **39 unique candidates / 10,884 bytes**, individually P/B/U-labelled:
36 named / 10,274 plus three independently attributed teardown bodies / 610.
U is attribution/size only; B specifies a published boundary and unclosed remainder.
Not a full-family census or 58/13,229 completion claim; no new record-byte credit.
Supporting queue/container helpers are measured separately, excluded from the owner
total. Foreign class implementation bytes are not credited as owned behavior.

Six **PROPOSED / UNEXECUTED** independent examples cover loaded generator+attractor,
multiple ownership sites, queue/controller close order, removal before close versus no
notification, empty/repeated teardown/final Release, and normal queue mutation.
They count direct operations under explicit normal-return/nonmutating callback conditions;
they provide no native pass or historical-warning adjudication.

## Checks and questions

- `python tools/doccheck.py check`: passed.
- Questions index regenerated (unchanged), `python tools/questions.py check`: passed.
- `git diff --check`: passed; ledger check: 39 unique addresses, sum 10,884 bytes.
- Cargo/clippy/ctest: n/a (docs-only RE). Game runs: none authorized or requested.
- AT21 retained with precise partial evidence in [questions/automata.md](../specs/questions/automata.md).
  No new IDs and no inference-based archive entry.

## Remaining scope

AT21 residuals: transitive controller Shutdown/ref effects (AT22), registration-service
internal retained references, and earlier city callbacks' possible RemoveOccupant
production. Loaded serializer ownership remains prior record input, not inferred extra
AddRef. Constructor defaults, complete overload signatures/filtering/creation admission,
other message branches, numeric time arithmetic, callback mutation and failure rollback
are unclosed. This lane requests no extra round and clears no park/cap/glue restriction
or existing native warning. Ready status applies to this coherent bounded publication.
