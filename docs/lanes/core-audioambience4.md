| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-audioambience4: pure-Rust cSC4AudioAmbienceManager owner (docs/specs/audio-ambience.md) | `core-audioambience4` / (engine lane) | in progress | see Handoff |

## Handoff
- Goal: engine/core/src/audio/ambience.rs, typed host trait, spec §1-§10 manager behaviour, conformance-style tests; no plugin/CAPI/CMake/ABI changes; game false.
- Done: worktree created from main 41bcc6d5.
- Next: write ambience.rs, register in audio/mod.rs, qbuild quick, commit; then questions AA5+; then qbuild all.
