| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lv28: land-value terrain retention and host eligibility (kind docs) | Codex / re-lv28 / reserved branch re-lv28; detached HEAD | ready; dependency merge 48661b4c6, spec commits 3ac71da53, 7c39e839b, 4f200fbb4; ABI/seam none; scoped docs PASS; cargo/clippy/ctest n/a; no new game run; supplied base1 clean/ref0 3/4 exact, inputs1 clean/ref0 1/4 exact, idle house 73/73 exact; LV9/LV10 answered | 1. Supply/validate a host before native capture. 2. Implement/validate vectors 32–36; preserve idle house exactness and obtain LV2/LV7 runtime records. 3. Review the inherited suite-list merge resolution when landing the dependency. |

- Wrote landvalue.md §§6.1.5–6.1.6: actual delivery observation, pre-fetch host admission, startup/shutdown eligibility, per-state terrain read order and retention.
- No extra terrain refresh: dirty does not rewind the running pass; later reads are live and the next eligible monthly pass refreshes work.
- Corrected fallback capture: state 3 resets counting but keeps first-scan min/max. Both scans are needed for faithful replay.
- Added controlled numeric fixtures and conformance specifications 32–36, plus the complete January/February repro and separate March 6 native expectations from both logs.
- Numeric analysis passed four float32 cases: intrinsic A=1 unchanged, 22 with live altitude/old statistics, 1 after ordinary refresh, 10 with fallback extrema retained.
- Landvalue-only questions check and full generated-index freshness PASS; whitespace PASS. Global questions.py check FAILS only NT36 (network-tools open/answered duplicate); both NT36 source files are unchanged from starting revision f4bf6b6e7.
- Moved LV9/LV10 to docs/specs/questions/answered/landvalue.md and regenerated the question index. LV2/LV7 stay open.
- Native capture remains ineligible: no supported host provision identified; pass-end-sample/native_step_capture=0. Reference-host fixtures and exact outputs do not install a game host.
- Existing measurements only: lvterrain27-repro-base1 and lvterrain27-repro-inputs1 are distinct clean/ref0 recordings; no new game, engine gate or capture run was performed here.
- Merged landvalue-terrain27 first as requested. The suite.tsv conflict used the current checkout's side (-X ours); its terrain script and other dependency changes were retained. No engine content was inspected or authored by this RE lane.
- Evidence came from the main checkout's local export/decomp, with numeric/read-order instructions checked using asm --bytes. No REA use or ABI proposal file.
