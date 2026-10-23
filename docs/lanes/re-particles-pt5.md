| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-particles-pt5: particle effect owners | Codex; re-particles-pt5 (detached HEAD) | ready; partial spec with explicit numeric gaps | Implement supported contracts and PO01–PO36; RE follow-up PT8–PT11 before full mesh/solver replacement. |

## Report

Extended [particles.md](../specs/particles.md) with §§13–19: owner inventory/completeness,
component dispatch and drawing boundaries, geometry-source ownership and parameter channels,
decal handle lifecycle/timing/placement, dynamic host message/tick/sector ordering, rigid-body
lifetime/noncontact integration, sequence-description slots/factory/IO, and particle-base copying.
Linked the detailed [coverage helper spec](../specs/coverage-radius-effect-helper.md) rather than
duplicating its construction, masks, refresh, teardown and literal traces. Added 36 proposed,
unexecuted conformance fixtures in the spec; no engine files read or changed.

Settled instruction-backed facts include dynamic message `86AD10EE`, unsigned microseconds,
binary32 0.1-second cap and maximum 0.005-second step; decal integer handle sentinel
`FFFFFFFF`, rectangle conversion, second-element size parameter and strict phase >1 stop;
geometry main/+4 receiver distinction; body old-velocity integration and primary-sector selection;
sequence version/flags handling and factory allocation failure. PE tables/constant reads support
the cited interfaces; local evidence helper is ignored `build-runs/pe-facts.py`.

The spec remains **partial**, and cannot yet drive an exact full replacement of all requested
rendering/physics domains. Filed **PT8–PT11** in
[questions/particles.md](../specs/questions/particles.md): geometry/draw/wave math, full decal
renderer records/random precision, dynamic static-sector/terrain construction, and exact 2D
contact/joint solver. Existing PT1–PT3 remain open; no archived answers changed. Questions index
regenerated. These are limits of the evidence reviewed, not measured runtime failures.

Validation: `python -X utf8 tools/doccheck.py check` passed at spec milestone and final review;
`python -X utf8 tools/questions.py check` and `git diff --check` also passed. Cargo/clippy/ctest
are n/a for this spec-only lane. No game runs, shadow verdicts or pixel-equivalence claims.
No C ABI exports changed; no proposed docs/abi rows. All original-program evidence used the local
export/decomp; REA was deliberately not queried because the brief says no binary is open.

Milestones: `7b73e60b` lane initialization; `2588c5d0` owner supplement, fixtures and questions.
Final report commit contains wording review and question append-order correction. Git emitted a
packed-refs.lock permission diagnostic during commits but produced the detached-HEAD commits
successfully; no refs/config changes or retry maintenance was attempted.

## Handoff

- Goal: reviewable partial owner supplement for eight owners, with exact numeric gaps identified.
- Done: §§13–19, 36 proposed fixtures, PT8–PT11; existing coverage helper linked.
- Next: coordinator lands this spec; follow-up RE must settle PT8–PT11 before claiming full replacement coverage.
- Evidence: export queries first; local decomp navigation and byte-bearing asm; PE table/constant reads in build-runs/pe-facts.py.
- Result deliberately partial: exact mesh/wave, renderer and collision solver remain explicit questions; no execution claimed.
- Main evidence root: C:/Users/tlori/FreeCity4; edits only in this worktree.
- No engine reads/edits, ABI edits, game requests, or agents.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-particles-pt5-r1-manifest.json.
