| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-my-street-sim4 — MyStreet native owner contract | Codex / re-my-street-sim4 | in progress | Audit prior owner accounting, then native lifecycle and overrides |

## Handoff

- Goal: document cSC4MyStreetSim's own contract, never ordinary MySim assumptions.
- Source: main checkout canonical Windows export; GOG 1.1.641. RE only; no engine reads.
- Prior audit so far: no history for docs/specs/my-street-sim.md; historical mysims1 and cores26 mention consumer/shadow attribution only. Continue exact dedicated-owner audit before claiming FIRST.
- Initial query finds named lifecycle at 0043D0F0–0043D1A0, constructor 0043D5F0, factory 0043DFA0, message 0043DFD0, attach 0043E0E0, trip update 0043D6C0.
- First substantive milestone: my-street-sim.md identity, lifecycle, serialization wrapper and 11 proposed UNEXECUTED traces. No copy/full inherited default claims.
- History audit found only generic consumer/shadow attribution, no dedicated owner contract; no reset of implementation accounting.
- Next: root hooks, balloon overrides, detach/attach, DoMessage and trip update; complete owned/shared/inherited/foreign ledger.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-my-street-sim4-r1-manifest.json.
- No native execution, game requests, implementation, ABI edits, or coverage credit.
