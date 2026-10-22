| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-overlays-contract4: native overlay owner contracts | Codex / `re-overlays-contract4` (RE docs) | in progress | Finish round partition and bounded Draw/model-shadow contracts; publish remaining methods. |

## Handoff
- Goal: native nine-float placement equations first, then constructor/handles/alpha/matrices/draw/round contracts.
- Worktree reserved, detached HEAD; main..HEAD empty and working tree initially clean.
- Done: 4912da47 publishes first exact record/zero-angle placement, tolerance and alpha vectors before call 30; OV6 answered.
- Done: native Init mutation order, getter identities, FIFO reuse, direct-ID exception, equal setters and slot21 exact matrix documented.
- Done: 4d11b61a publishes Init/getters/FIFO/slot21; ordered rotated coefficient table, overlay defaults, clipping/index subset added; OV4/OV5 answered.
- Next: OV1 shadow args/post-transforms (slots9–11), OV7 camera inverse/composition, OV8 draw/counters/visible-row endpoints, OV9 eight-part partition.
- Key corrections: slot22=shown, slot26=self-lit; slot24 output first; slot21 u=(x+z)/size, v=-y/size+1+offset/size; no group recursion.
- Read overlay spec sections 2/3/4 and open OV1-OV9; OV2/OV3 outside priority unless needed.
- q.py cannot launch Bash (E_ACCESSDENIED); use capped native PowerShell and re_query.py directly.
- Edit docs only; never read engine; ABI proposals only under build-runs/pending-abi, with exact rows also in abi-rows file.
- No game requests or runtime gates required for docs-only work; no verdicts claimed.
- REA batch (PlaceOverlay/ComputeCellRect/SplitDiscIntoParts/AddDecalFromBinding) failed target_unavailable; export/bytes fallback used.
- Manifest (active round 2): C:/Users/tlori/FreeCity4/build-runs/run4-re-overlays-contract4-r2-manifest.json.
- Done: 1ebda27a adds rotated rounding, defaults, clipping/index rules; OV4/OV5/OV6 archived, OV1/OV7/OV8 narrowed, OV2/OV3/OV9 preserved.
- Validation: questions.py index/check passed; git diff --check passed at all three milestones. Runtime cargo/clippy/ctest/game n/a (docs only).
- Guard reached OVER 150000 after a9a8a5c4; continuation required, no merge-queue record.
- Exact next step: query 0073c0d0 (model shadow), 0073b3b0 post-placement and 00738a50 Draw camera setup/callees; finish OV1 before OV7/OV8/OV9.
- Round partition entry 007394b0: shipped globals B0D4D0=.4000000059604645, D4=.20000000298023224, D8=.10000000149011612; A92D28 is double .5. Not yet a proven partition.
- Native scratch: build-runs/native-data.py (PE constant reader), place-native.txt (only first56 asm lines; do not mistake for full routine). Never commit these.
- ABI: no export changes, no ABI rows/proposal required in this milestone; docs/abi untouched.
- Git commits succeed detached despite nonfatal packed-refs.lock permission diagnostic; supervisor must publish branch. No branch/config/ref operations performed.
- Last unincorporated proof: DrawDecals empty input exits at 007367A6 before resetting D8 at 007367E4; both shadow routines likewise exit on empty vectors (00737886/00736C0A). Dynamic snapshot Draw exits at 00738A76. Complete counter contract still pending.
- Round 2 milestone: §3.5 proves slot9 key/placement/flipV/secondary matrix and slot10 three-word key; §4.4 proves empty/disabled counters, masks and increment sites. Commit recorded in next milestone.
