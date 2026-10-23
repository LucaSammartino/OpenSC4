| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-app-directors5: application bootstrap directors | Codex / re-app-directors5 (detached HEAD) | ready: bootstrap spec and 27 vectors; doccheck passed | Implement linked contracts; investigate ADIR1–ADIR3 for remaining owner boundaries. |

## Report

Added [app-directors](../specs/app-directors.md) for cGZToolsCOMDirector,
cSC4GameDataRegistry, cSC4ResourceLibCOMDirector, cSC4COMDirector and
cGZResManCOMDirector. The document links the existing Tools and registry owner
contracts rather than duplicating their service algorithms. Application §12 now
links the new bootstrap contract. Milestone: `c7c75670`.

Specified shared director state/defaults, all 17 primary slots and the 11-slot
secondary face, identity/refcounts, initialization, flattened child insertion,
class lookup/enumeration, unload behavior and destruction. Recovered the game
constructor's 14 subordinate-director sequence (ten immediate Starts) and all
190 own class/factory registrations. ResourceLib's 25 registrations link the
existing editor inventory; ResMan's 16 are listed explicitly. Included service
registration, resource-type binding, drawing/IO delegation and failure boundaries.

Important observed distinctions: ResourceLib Start uses stored COM while ResMan
Start uses its argument; service outputs are tested independently of COM Boolean
results. ResourceLib's binding hook returns false without the manager, but the
framework hook dispatcher ignores that return. Factory-null is an unchecked
native dispatch boundary, not a proved false result. Tools hooks actually occupy
eight lifecycle slots (+0C through +28); corrected the prior off-by-one claim and
its vector in [tools-com-director](../specs/tools-com-director.md).

Validation: `python tools/doccheck.py check` passed; `python tools/questions.py check`
passed; `git diff --check` passed. Local read-only evidence checker confirmed
190/190 Appendix A pairs against export instructions/native immediate getters,
190 unique IDs, 27 consecutively numbered conformance vectors, and all four
44-byte hooks-table spans. Evidence helper is ignored under
`build-runs/directors-evidence.py`; no RE material was committed. A provisional
helper interpretation of the eleven literal-ID registrations was corrected before
the spec inventory was written; only the corrected inventory was validated.

Questions: ADIR1–ADIR3 filed in
[questions/app-directors](../specs/questions/app-directors.md); index regenerated.
They cover exact local hash enumeration, three singleton exception handlers, and
remaining factory-owned interfaces (including class 69B6F01C). These are explicit
limits, not claims of complete reachability-based owner coverage.

No engine/, plugin, capi, tools/coverage-map.tsv, docs/abi or C ABI changes.
No proposed ABI rows. Cargo/clippy/ctest and game/shadow runs: n/a for this RE
documentation lane. No REA calls were attempted because the brief identifies no
open binary; all evidence came from the local export, native bytes and decompilation
navigation. The detached commit succeeded despite a packed-refs.lock permission
diagnostic; no shared-ref/config/maintenance action was attempted.

## Handoff

- Goal: specify five bootstrap directors/registries in docs/specs/app-directors.md.
- Done: spec milestone c7c75670; 190 verified game pairs, shared slots, resource lifecycle and 27 vectors.
- Done: Tools hooks extent corrected; ADIR1–ADIR3 indexed; doccheck/questions/whitespace checks passed.
- Next: coordinator publishes detached tip and lands the ready RE queue entry; implementers use linked specs.
- Remaining RE: exact hash order, exceptional getter retry and uncontracted factory-owned interfaces.
- Facts: no game requests outstanding; ABI unchanged; no coverage-map edits; no engine files read.
