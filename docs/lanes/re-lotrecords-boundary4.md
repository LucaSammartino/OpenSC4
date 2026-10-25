| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotrecords-boundary4: LR9 views and LR8 native boundary | Codex / `re-lotrecords-boundary4` (RE, detached worktree) | ready: `81a2532b`, `ebe0d279`; LR8/LR9 answered; 5 discriminating vectors; 4 ABI row corrections proposed; questions/check and diff/check passed; cargo/clippy/ctest and game n/a | 1. Review/apply/gate pending ABI corrections. 2. Plan developer-inclusive replacement; LR6/LR7 remain open. |

## Result

Updated `docs/specs/lot-records.md` sections 3.1, 4, 7.1/7.2, 13.2 and 15, with native boundary vectors
T26-T30 in section 17.4. Initial source/target proof and vectors were committed in `81a2532b` before call 30;
ownership details, proposals and archived answers followed in `ebe0d279`. No engine or tools sources changed.

LR9: lot property holder remains +8; traffic source remains +0x1C. Primary slot 3 is AsPropertyHolder
(`006c2040`), slot 4 AsTrafficSource (`006c2050`); both borrowed, null-preserving casts, EAX pointer,
no stack arguments. The shared target's old property-holder label on slot 4 is wrong. Native constructor
writes and traffic-source iterator `00710841` prove the association. QI accepts only IID 1, lot and
Serializable; all secondary QI/refcount thunks adjust back to the common owner. Failed QI leaves output
untouched. Evidence is instruction-verified, not inferred from the current implementation.

LR8: zone-developer lot-manager slot 23 (`0072e520`) directly allocates from the lot pool and calls the
five-word constructor `006c1e50`; output and master list acquire separate references, cell pointers do not.
PickUnitToJoin directly constructs the zero-count business unit, with member lots retaining it. Native
ClearLot/AddLotFoundation and slot 62 call wall helpers directly; AddRetainingWallOccupant directly constructs
the wall, retains its lot, inserts the occupant and releases its local creation reference. The spec records
exact caller/target addresses, cleanup, return channels, storage and reference obligations.

Factory registration covers serialized-object creation but cannot redirect these play-time edges. The
specified complete-owner boundary therefore requires replacing the zone developer/lot manager for lots,
and the lot developer with business-unit and wall operations. No supported standalone installation or
restoration contract for direct-edge interception was found or invented. This is a replacement dependency,
not a claim that machine-code patching is impossible or an approval of a plugin installation mechanism.

## ABI proposals

Four existing rows only (slots 3/4 in each table), with unchanged keys/targets/stack ABI; class-specific method
labels and pointer return confidence corrected. No C ABI exports changed. Coordinator review/application/gate
is required; `docs/abi/` was not edited.

The proposals are now published in `docs/abi/interfaces.tsv` and `docs/abi/vtables.tsv`
(vtable `00ab19b0`, slots 3/4). The duplicate scratch rows and application rationale
were removed after checking their contracts against both published tables.

## Validation and limits

- `python -X utf8 tools/questions.py index` regenerated the index; `questions.py check` reported `questions: ok`.
- `git diff --check HEAD~2 HEAD` passed after removing a trailing blank line.
- Proposal validation found exactly two unique existing rows per table, valid TSV shape, unchanged key/target/
  cleanup fields and an identical interface mirror. Validator is local scratch in `build-runs/`.
- Five vectors are source-derived conformance requirements, not executed native game tests. No game runs,
  shadow comparison, cargo, clippy or ctest were applicable to this documentation/proposal-only lane.
- Used main-checkout `re_query.py` export queries first, bounded decomp reads and native PE view-table reads;
  numeric widths/offsets and cleanup verified with `asm --bytes`. REA was not needed or called.
- q.py's Bash launch was denied by the environment; bounded direct PowerShell/Python checks succeeded.
  Git printed a nonfatal packed-refs lock denial, but both detached commits succeeded; no refs/config changed.

## Questions and next steps

LR8 and LR9 moved to `docs/specs/questions/answered/lot-records.md` with dated MATCHES/DIFFERS rules;
index regenerated. LR6/LR7 remain unchanged in `docs/specs/questions/lot-records.md`. No new questions filed.
Coordinator should review/apply/gate the scoped ABI proposals and use the developer-inclusive boundary for
future runtime work. These docs do not enable standalone lotrecords registration or grant runtime replacement
credit. Detached HEAD is intentional; supervisor publishes the commits.
