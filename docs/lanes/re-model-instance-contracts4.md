| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-model-instance-contracts4: portable model contracts | Codex / `re-model-instance-contracts4` (RE) | in progress | 1. Prove MI3–MI10 from original instructions. 2. Publish contracts and independent vectors. |

## Handoff

- Goal: resolve MI3–MI10 for the published portable8owner subset; preserve opaque MI1 and literal MI2.
- Worktree is reserved and detached; commit normally, never switch branches or update shared refs.
- Done: proved MI3/MI10, documented defaults/record packing, byte counter sources, distinct basic/static count widths and owner-event order; five independent vectors. First substantive milestone in this commit.
- Next: finish sprite resources/factory, then processor sorting/lifetime, bounds/picking and malformed/numeric limits.
- Facts: properties default 2; draw +2..3 unwritten; counter u8; basic ref u16, static ref u32, both destroy at count <=1.
- Main evidence root: `C:/Users/tlori/FreeCity4`; edit docs/specs and this lane file only.
- `q.py` cannot launch Bash (E_ACCESSDENIED); use bounded PowerShell commands directly.
- No ABI exports, game runs or runtime gate needed for this documentation lane.
- Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-model-instance-contracts4-r1-manifest.json`.
