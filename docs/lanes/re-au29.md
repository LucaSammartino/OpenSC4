| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-au29 audio AU42/AU43/AU50; kind docs | Codex; re-au29 worktree; reserved branch re-au29; detached HEAD commits | ready; 5df811904, a96ab9c96, d05d852c9, 6ecf2996c plus finish; doccheck passed; diff check passed; cargo/clippy/ctest n/a; game n/a; shadow n/a; ABI false; AU42/AU43/AU50 answered | 1. Coordinator enqueue kind docs and land. |

Completed 2026-10-26. Spec text only; no engine, ABI or tool changes.
AU42: audio-scape §§3.5, 6.2, 7.4–7.5 and 11 complete format/ownership, timer widths,
channel/cache smoothing, capacity outputs, grid arithmetic and allocation requirements.
Slot 5 copies a name string; this owner has no scape resource decoder.
AU43: audio-preferences §§3.9, 6.6, 7.1 and 9.1 complete discovery/persistence,
construction/close ordering and worked control cases. Visibility, not enabled state, gates sources.
OK saves before list rebuild; Cancel restores preferences without rebuilding the radio lists.
AU50: audio §8.3.2 completes voice admission, last/first instance selection, linking,
pre-insertion-count gain, refresh order, zero sentinel bits, promotion and payload ownership/results.
Existing audio-owner-closure §7 supplies the accepted instance-side arithmetic/pause/Resume bodies.
Target entries moved to questions/answered/audio.md with dated answers; questions index regenerated.
No target runtime observation remains required; unrelated questions remain untouched.
Gate: python -X utf8 tools/doccheck.py check passed; git diff --check passed.
The q wrapper failed before executing its check (Bash CreateFileMapping, Win32 error 5);
the separate native PowerShell doccheck completed successfully.
Git emitted packed-refs.lock permission warnings; each detached-HEAD commit succeeded.
REA unused; export/decomp and instruction-byte evidence sufficed. No proposed ABI rows.
Game runs, shadow comparison, Rust/C++ tests: n/a for docs-only work.
