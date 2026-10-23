| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-text-decal-owner4 (city-detail text decal static owner contract) | Codex / `re-text-decal-owner4` (reserved detached worktree) | ready bounded docs publication; owner partial for arbitrary-input FP bits; inherited FINAL3/3 exhausted | Land docs; retain TDO1; no further child round authorized. |

## Report

Published [TextDecal owner contract](../specs/text-decal-occupant.md) and
[literal proposed vectors](../specs/text-decal-vectors.md). Replaced lot-records §10.4's
contradictory mixed-frame tables and corrected its stale §16 questions/quirks. Added the
dedicated contract to occupants-props' public index. No engine, tests, ABI tables or tools changed.

Native receiver proof: complete O owns class setters/internal helpers; embedded prop O+4,
message O+8, occupant O+12, serializable O+16, holder O+20, exemplar holder O+40.
SetPosition's +88/+8C are O+94/+98; Shutdown's +AC is manager O+B8, whereas orientation
setter's +AC is O+AC. Record +5C is O+6C text; repeated pair is position, not IDs.
OP1 moved to answered; OP2-TD archives the TextDecal-only stream answer, retaining signpost OP2.

The contract owns source replacement and failure, manager reference replacement, exact remove/add/
color/alpha/move order, size/aspect/box computation and turns-table lookup, flags, Init/messages,
Shutdown versus destruction and stream alias/version behavior. Key corrections: color difference
**equal** to epsilon triggers; height 8 rejects; equal accepted height rebuilds; failed source
replacement releases old source but leaves existing IDs; manager routing selects B index1/3
then A index0/2. No guessed rollback, null-manager fallback or invalid-storage safety.

Static identity measured from the canonical read-only export and PE: FileVersion 1.1.641.0,
SHA-256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
Used re_query/export instructions first, local decomp only for navigation; no REA query required.
Verified raw constants and 16 binary32 turns values without loading/executing the original.
The 36-entry separated native-body/thunk/folded checklist and foreign view mappings are local
evidence, **not** the old 35/3545B attribution denominator or complete class-byte credit.

All vectors are **UNEXECUTED** independent literal proposals. No game/debug/original execution,
live-save trace, implementation output, shadow or game acceptance was requested or fabricated.
Cargo/clippy/ctest: n/a (docs-only). Validation: `questions.py index`, `questions.py check`,
`doccheck.py check` and `git diff --check`; final verification recorded in the closing commit.

Open: **TDO1**, `docs/specs/questions/text-decal-occupant.md`, global x87 precision/rounding
environment across admissible callers. Owner status remains partial for arbitrary-input exact
bits; explicit store points, normal-domain decisions and environment-conditioned vectors are
published coherently. Truncated-record scratch and invalid table/storage cases have no invented
deterministic fallback. No ABI-row proposal; no C ABI export change.

Milestones: setup `ba05bd65`; first substantive native OP1 contract `cf4e6f8f` (before call25);
owned algorithms `53a1c038`; vectors/questions/coverage `314a39d2`. Detached commits succeeded;
Git also printed packed-refs.lock permission warnings, so no ref/config workaround was attempted.

Reservation: copied strict proof in `build-runs/evidence-run4/` records parent two rounds plus
this sole child FINAL3/3. Manifest `C:/Users/tlori/FreeCity4/build-runs/run4-re-text-decal-owner4-r1-manifest.json`.
This consumes the only authorized child invocation; no continuation/new alias/cap borrowing.
Next: coordinator may land this bounded docs publication; retain TDO1 and signpost's separate OP2.
