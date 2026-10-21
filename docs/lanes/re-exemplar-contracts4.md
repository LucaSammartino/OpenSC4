| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-exemplar-contracts4 (answer EX46-EX50, data-formats §4.4/§8) | `re-exemplar-contracts4` / (RE spec lane) | ready: spec-only; EX46-EX49 answered (DIFFERS) in data-formats §8.2 + §10.8; EX50 partly answered, stays open; no ABI, no game runs | runtime caller trace of exemplar GetPropertyList (return addresses) to close EX50; glue lane reconciles stub choices with §8.2 |

## Report

REA had no app open, so everything is static (re_query asm/callers, decomp/, raw vtable dumps with
build-runs/vt.py). Commits: 72e48530 (lane file), 4a9bd192 (facts), 4ca9a30b (spec §4.4/§8.2, questions), then
the conformance §10.8 and this report.

What changed (docs/specs/data-formats.md)
- §4.4: GetPropertyList inserts with list slot 7 = push FRONT, so the list runs in descending ID order; occupant
  EnumProperties visits own (ascending) then the exemplar list (descending). New paragraph on the callers (EX50).
- §8 table corrected: exemplar/cohort slot 14 is DebugInstanceTag(cIGZString&), not a rebuild.
- New §8.2: serializable stubs 3-5, slot 14, slot 15 rebuild(vector*), parent ownership (borrowed Get, AddRef/
  Release Set, same pointer no-op), removal (own list, then parent's property substituted in the effective list),
  holder slots 22-24 (deleting-dtor thunk; two return-false stubs), change messages (data1/data2 = key words
  +4/+8, SetExtra(object), MessagePost(msg,false); no caller found passes true), the unknown-list class (vtable
  0x00ACE650, 19 slots, no GZCOM class ID, QI 0x12E/1, slot table), factory slot 8 and SerialWrite formats.
- §10.8 conformance: list order, remove restores parent, SetCohort references, stubs.

Questions
- EX46, EX47, EX48, EX49 moved to answered/data-formats.md, all DIFFERS from the lane guesses in at least one
  point (slot 14 is not a rebuild; list is descending; messages carry the object as Extra).
- EX50 stays open with an update: static callers are occupant EnumProperties (0x005FEE10/0x005FEF00), the
  original rebuild (parent cohort; not run on replacement objects) and the developer exemplar editor. Unknown:
  who calls occupant EnumProperties and the in-city action. No reproducible in-city action is claimed.

Implications for enable=exemplars (for the glue lane, not rationalised here)
- A replacement GetPropertyList must return descending order if a caller depends on order; readers seen use
  only count, PopFront(0x452160EC) and Release.
- Holder Remove* must re-expose the parent's property; SetCohort must AddRef/Release and skip a same pointer.

Next
- Runtime: log return addresses of exemplar GetPropertyList calls in a union run to close EX50.
- Static: trace callers of occupant holder slots 19/20 (EnumProperties) and the manager save slots 12/13.
