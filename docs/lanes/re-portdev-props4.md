| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-portdev-props4: PD12 native seaport prop switching | `re-portdev-props4` (RE; reserved detached worktree) | ready; PD12 answered; docs only; no C ABI change | Coordinator: wire prop host using section 4.4.1, review optional ABI return annotation and validate replacement. |

## Report

PD12 is answered in `docs/specs/port-developers.md` section 4.4.1 (proof/contract commit `f017a7ac`). The
original question was moved to `docs/specs/questions/answered/port-developers.md`, marked DIFFERS, and the
question index regenerated: port-developers now has zero open and twelve answered questions. Initial scope
commit: `e886822d`. Work stayed on the supervisor's reserved detached HEAD.

The native monthly tick sets the inaccessible-port advisor flag before calling `SetLotPropsState`.
`0x00701A60` calls `GetLotProps` at `0x00701190`; this obtains the inclusive lot rectangle, converts its
corners through city slot 96, and queries occupant-manager slot 40 / +0xA0 with filter `0x007010E0`.
The filter requires prop type, position within inclusive world x/z bounds and occupant-holder HasProperty.
The property value is never read; the ordinary holder includes instance properties and exemplar fallback.
This is a spatial query, so owning-lot identity does not decide membership. Manager inset-box intersection
is an additional prerequisite, documented separately from the filter's inclusive boundary comparisons.

The returned native list owns one occupant reference per node. Finished-list order and multiplicity are
preserved; the prop interface is acquired with IID `0xE9793A65`. Its SetState is zero-based slot 11 / +0x2C,
x86 __thiscall, one 32-bit argument, callee pop 4, void. Acquired interfaces remain held across the call;
manager slot 48 releases occupants and list nodes, and the local destructor releases the sentinel.
There is no native count return. An adapter diagnostic count of calls issued is explicitly a choice;
duplicates and already-1 props count, null interface outputs do not. It cannot gate the advisor event.

Twelve independent constructed vectors specify empty/multiple results, order, instance/exemplar property
presence including false/zero/empty values, type and spatial rejection, unrelated owning lots, null and
contradictory QueryInterface outputs, duplicate list entries versus native grid deduplication, enumeration
failure, missing providers/holders, and lifetime across manager-membership changes. Missing native providers
are unchecked preconditions, not a native empty-list result; a defensive host fallback must be labeled.
These are written expectations, not executed host tests or new runtime evidence.

Evidence used the local export first (`re_query.py asm --bytes`, paged at at most 56 lines), then direct
read-only PE vtable words to connect unnamed filter/manager/prop bodies. Main helper bodies: 334-byte
`0x00701A60`, 257-byte `0x00701190`, 164-byte `0x007010E0`. Manager bodies audited:
`0x0049E250`, `0x004A0670`, `0x0049EF30`; prop constructor/QI/SetState and holder HasProperty also checked.
REA procedure_pseudo_code returned `target_unavailable` (no open app); used the export/binary fallback,
without opening or changing the shared REA target. q.py could not start Bash (E_ACCESSDENIED), so commands
used bounded PowerShell output. No engine files were read or edited; no tools or ABI tables were edited.

ABI proposal only: `build-runs/abi-rows-re-portdev-props4.tsv`, in vtables.tsv format, exact existing key
`cSC4PropOccupant::vftable / 00aa4900 / 11`; proposes the proved void return and state-word argument
annotation at target `005ed290`. Evidence is embedded in the row. Coordinator review required before applying;
no ABI version or C exports changed. Temporary PE reader: `build-runs/pd12-pe.py` (not committed).

Validation: regenerated question index; checked the question/index diff and documented vectors against
native branches, references and vtable bytes. Final whitespace check recorded in publication commit.
Cargo/clippy/ctest and game runs: n/a for this docs-only lane. Existing glue-portdev4 evidence remains
144/144 shadow exact and clean/ref0 replacement to 2032; this lane makes no additional runtime claim.

Next: coordinator can wire the already-built prop host using section 4.4.1, review the optional ABI
annotation, and exercise the independent vectors plus a seaport replacement scenario. No RE question
remains open for PD12. No implementation, merge, push, or restart was performed.
