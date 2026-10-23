| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-local-grid-helper4: portable local grid helper | Codex / core-local-grid-helper4 | in progress | Implement literal helper and independent public vectors; canonical gate |

## Handoff
- Goal: first production LocalGridHelper, lifecycle and five-scalar/two-floor transform, no ABI/plugin wiring.
- Start HEAD: 15aced92; reserved detached worktree reused; initial tree clean.
- Read: rules, lane format, local-grid-helper sections 1–6 and open LGH1.
- Independently transcribed G1–G7 (28 words), seven B cases, L1–L14 in tests/local_grid_helper_vectors.md before implementation.
- Production: only persistent H; direct factory output; fresh AL/low-byte control; clear-before-release shutdown; consuming release-only destroy.
- Production arithmetic: exact bounded f64 intermediates and integer nearest-even R32, held-scale comparison, 54-defined-byte projection.
- Tests written: public G/B/L ledger, direct output address, ties, domain extremes, unsupported prefixes. Domain proof in engine/core/tests/local_grid_helper_domain.md.
- Formatting passed with absolute owned .rs paths (qbuild fmt exit=0); relative paths failed due wrapper working directory. No shared lib reformat.
- Next: focused owned tests through quiet wrapper, then commit stable source and one canonical qbuild all.
- q.py single-string Bash invocation fails E_ACCESSDENIED; use direct argument-vector invocation for focused commands.
- Numeric mode must include G7 held nonunity/stored unity; unsupported inputs retain reached primitive prefix.
- No game requests; portable core scope only. No baseline per task-specific gate instruction; one canonical all after focused tests.
