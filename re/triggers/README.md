# Executable trigger map — RE census, 2026-10-25

## Round 2: owner activation

The owner census supersedes the function graph as the scenario-planning unit.
`owner-population.tsv` groups every implemented entry by the same attribution
owner that `ledger.py` writes, preserving method-specific coverage-map matches
when deciding which entries count as implemented. It includes 468 owners,
14,790 entries and 3,361,604 implemented bytes. Of these, 895,760 bytes have a
positive entry in the pinned provisional reach pilot. Exactly 185 owners have
at least 2,048 implemented but unobserved bytes; their gaps total 2,333,689 bytes.
The remaining 283 owners account for 132,155 unobserved bytes.

`owner-inputs.tsv` pins the current main-checkout inputs used for this round.
The attribution and implementation totals were recomputed with the existing
coverage rules; implementation paths were checked for existence only. No
implementation source was read. Main still lacks `docs/abi/reach.tsv`; round 2
therefore retains the exact provisional pilot pinned below. An owner being
activated is not a prediction that all of its unobserved methods execute.

## Round 1: retained function-level candidate graph

**Status: WIP. This is a structurally incomplete candidate map, not a validated
work order or a dead-code list.** The idle-path control fits only 7/30 sampled
entries. Resolving receiver types, QueryInterface results, callback tables and
branch preconditions remains necessary before using it to claim reach coverage.
No implementation or ABI contract is changed by these tables.

## Inputs and population

The population is every address in the main checkout's `docs/abi/functions.tsv`:
35,648 entries, 6,509,027 body bytes, including library/shared entries. Addresses
are executable virtual addresses, lower-case hexadecimal without a prefix;
sizes are the ABI inventory's decimal body sizes, not distances to the next entry.
Implementation attribution follows `coverage.py`: its existing owner propagation,
method-first map matching and nonempty core list with all paths existing. Only path
existence is inspected under `engine/`; implementation source is not opened.
14,790 entries / 3,361,604 bytes count as implemented by that rule. This does not
claim a separately measured implementation for each method of a mapped owner.

The reach input is pinned to `reach27` commit
`de1360e4ef01d9558f6b36cb85e13a0ba43e8fce`, `docs/abi/reach.tsv`. Its own header says
**PROVISIONAL integration pilot, NOT canonical house**: `reach27-pilot-on-2`,
Konradshohe, four pricezone actions, 2026-01-27 through 2026-05-01, five partial/full
windows, original simulators, load/shutdown/probe intervals excluded. Consequently
the required `reached_house` column currently means positive entries in **that
provisional input**, not an established idle-house result. Zero is lack of an
observation, not evidence that a function cannot run. The input records 7,021
positive entries, covering 895,760 attributed implemented bytes (26.65%).

`inputs.tsv` pins hashes of the eight principal static inputs. The complete reach
source and analysis intermediates are in this worktree's `build-runs/triggers/`. Scratch drivers are
`build-runs/trigger_inventory.py`, `trigger_build.py`, and `trigger_controls.py`;
they are RE-only generated-data tooling, not implementation deliverables.

## Table meanings

`triggers.tsv` has the requested nine columns and exactly one row per input
address. A primary root is the shortest available path, with deterministic ties
by non-low root confidence, kind and label. This choice does not suppress the
existence of other roots: `nearest.tsv` gives the nearest discovered root of each
kind, plus one predecessor/callsite witness. Its predecessor is a witness for
that row's root; it need not itself select the same root in its own nearest row.

`roots.tsv` records entry addresses, dispatch IDs when available, and the basis
of each root. `registrations.tsv` links 92 candidate command/cheat names to their
registration references and receiving handlers. Names are the executable's
registered strings, including case and spaces. Registration is separate from
proof that valid parameters or a reachable UI can invoke the command.

`actions.tsv` ranks 999 candidate roots by exclusive bytes, then total bytes,
kind and label. For each root, total is the sum of implemented, provisionally
unobserved entries in its discovered transitive call set. Exclusive counts entries
in that set and in no other candidate action set. Startup/error roots are excluded
from that competition. City persistence and conditional tick roots are included
because a scenario can supply a save/object state or let the clock run. These are
**may-reach byte estimates**, not guaranteed new entries or independent marginal
gains. Missing edges can both reduce totals and artificially increase exclusivity.
Each function's bytes are counted once within an action; different actions overlap.

Confidence is about the structural evidence, never dynamic exactness. `exact`
is reserved here for the PE entry. `high` identifies a named/registered callback
or a direct-call structural path; it does not prove branch feasibility. `low`
includes inferred receiver types, generic callback conditions, candidate UI IDs,
message-ID push sites and unresolved roots. Root registration confidence and the
confidence of every downstream action estimate are different; even a high root's
total can contain low-confidence downstream edges.

## Graph construction and known gaps

The graph uses direct export calls and decoded jumps to other function entries.
All callsites of a repeated direct target are retained for dispatch filtering.
Constructors' installed vtables, block-local register aliases and slot offsets
resolve some indirect calls. A separate low-confidence pass infers singleton
stores and simple named getter return/field types. It never connects every call
at one slot number to every vtable with that slot, and never treats installing a
vtable as execution of all its methods. It resolves 670 indirect callsites out
of 81,237 decoded indirect calls; this is far from receiver-complete.

For command/message roots the first entry `GetType` call at slot `+0x10` seeds a
constant-ID abstract branch traversal. Known unsigned comparisons narrow branches;
unknown state branches retain both alternatives. Only outgoing calls at visited
sites enter the root's traversal. Computed jump dispatch and message types not
recognized by this entry pattern are incomplete. Nested callees retain the union
of their ordinary branches. A condition on the root therefore does **not** yet
carry every downstream population/funds/year/ordinance/building/disaster guard.

Root candidates include explicit app/framework callbacks, named city persistence
callbacks, simulator callbacks, named tool input/placement callbacks, registered
command/cheat dispatches, known message IDs and large constants in window handlers.
The last are **candidate message/control IDs**; their exact button/menu role is
not established. Message `PUSH` references list potential posters or subscribers,
not proven posts. Full constructor/subobject/IID propagation, registration-table
callbacks, validated UI IDs/posters and Mac/debug-only classification are missing.
Error roots currently cover recognized structured-exception handler registration,
not the complete allocation/file/invalid-data failure paths.

`none` means **no discovered path in this partial graph**. It is never sufficient
to classify an entry as dead, Mac-only, debug-only or failure-only. There are
30,413 such entries / 5,061,669 bytes, including 2,443,135 implemented bytes.
`no-root-review.tsv` checks ten small examples against instruction bytes; eight
are actually observed in the provisional run. This directly demonstrates missing
edges rather than dead code. Folded/automatic method labels also require care:
the accessor at `004778a0` returns an address, and `0090a929` forwards a range;
their imported names alone do not establish the advertised high-level behavior.

## Controls and scenario status

`house-control.tsv` selects 30 positive-entry addresses by ascending SHA-256 of
`triggermap27-control-` plus the eight-digit address. Selection does not seed or
modify roots. Only 4/30 primary kinds fit startup/city/sim-tick; allowing another
discovered root of those kinds gives 7/30. **All 23 remaining disagreements are
listed explicitly in that table.** This control fails, and the non-idle provisional
source further limits its interpretation. `totals.tsv` gives all disjoint primary
root-kind byte totals, including zero implemented error bytes.

`scenario-plan.tsv` keeps the top 15 candidates in ranking order and separates
eight script hypotheses from seven missing drivers/preconditions. The scripts
contain only public actions, cells, dates and values. They are not added to the
shared suite manifest: a suite owner must choose suitable cities and verify the
preconditions first. A load hypothesis depends on suite setup; its dated script
does not itself load a city. Placement attempts on occupied or unsuitable cells
can fail. No predicted set is a guarantee of successful placement or growth.

Dynamic validation is **not performed**: `main:docs/abi/reach.tsv` was absent at
inspection, and this reserved detached worktree cannot create/update scratch Git
refs. The reach branch was neither merged into this lane nor used to claim a new
run. Predicted/entered precision and recall are unavailable, not zero. There are
no new shadow/replace results or mismatches to report. The required complete
top-15 scripts and live prediction control therefore remain unmet.

Next work: resolve registered city/simulator interfaces and their IID/subobject
returns; rerun the fixed 30-entry control; recover each leading action's actual
branch condition and UI/message producer; then run matched baseline/action
scenarios with a verified reach build and distinguish entry observations from
incremental reach. Only after that should the ranking be accepted as a suite work
order. No REA calls were needed; no REA error fallback occurred.
