| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-pedestrians-oneshot13 (PN1 one-shot play-count consumer) | `Codex` / `core-pedestrians-oneshot13` (engine lane, detached HEAD) | in progress; implementation and 22 vector tests written; explicit-file fmt PASS | 1. Collect focused Cargo operation. 2. Run qbuild all and queued verify. 3. Commit final report and enqueue. |

## Handoff
- Goal: close cSC4PedestrianOneShot sections 3/7 consume_play_count and all owner closure rows.
- Done: setup committed as 9b09c608e; read answered PN1 and pedestrians sections 2.1/3/7/8.
- Done: implemented byte count/default 5, null-safe SetSpeed, unsigned wrapping RNG span, accessors, own tick and first-live final-frame consumer.
- Done: added 22 individually named PN1 tests; README closure rows have no open entries and map every vector.
- Done: qbuild fmt PASS on the three owned Rust files (absolute paths required by wrapper).
- Next: collect focused Cargo test operation, then qbuild all and qbuild verify on stable final source.
- Constraints: core only; no plugin/capi/ABI/coverage-map edits; use apply_patch for every file mutation.
- Focused operation: exec session 99323, cargo test --manifest-path engine/Cargo.toml -p fc4-core pedestrians; log build-runs/pedestrians-focused-1.log. Still compiling at this update; collect same session.
- Diagnostics: q.py could not start Bash (Win32 mapping error 5); direct Cargo used instead. Relative fmt paths failed, corrected to absolute; final fmt exit=0/diff_lines=0.
- Gates: final qbuild all and queued qbuild verify not yet started; no pre-edit baseline build.
- Game runs: none; pure core consumer task.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-pedestrians-oneshot13-r1-manifest.json.
