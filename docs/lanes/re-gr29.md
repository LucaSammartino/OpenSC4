| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gr29 / graphics-rules GR9; kind docs | Codex / re-gr29 / reserved branch re-gr29 (detached HEAD) | ready; prerequisite merge 3506382bb, specs dab649e0c + 9b4a4a6c7; ABI/seams: none authored; scoped docs 1/1 and whitespace PASS; global questions FAIL on inherited NT36; cargo/clippy/ctest n/a; game n/a; shadow n/a; GR9 answered | 1. Coordinator reconcile prerequisite build-registration conflict. 2. Engine lane implement providers and test off/on acquisition; existing GR4/GR6/GR7/GR8 limits remain. |

GR9 is answered in graphics-rules §§2.3–2.6 and archived in questions/answered/graphics-rules.md; question index regenerated.
Specified the FSAA current-target setup, 256 format-1 vertices, twenty draws, readback IID/slot, 32-bit buffer, samples, and UI-default cleanup.
Specified backing-store presence via the public clamping setter/getter with restoration; an uninitialized public renderer factory instance supports acquisition before a view exists.
Specified Win32 executable-resource/OS strings and CPU/memory acquisition, including timing arithmetic; no version-service IID exists. Specified sound service/IID and slot 25's two foreign string outputs.
Worked cases are source-derived fixtures, not GPU observations or implementation acceptance evidence. No native helper/manager/log callback is permitted as the provider algorithm.
Validation: build-runs/check-gr9-docs.py PASS (four open IDs, five unique answered IDs, index and memory examples); git diff 3506382bb..HEAD --check PASS.
Global tools/questions.py check exits 1 only for pre-existing questions/network-tools.md:120 NT36 also answered; no unrelated question files changed.
q.py validation attempts failed before execution with Git Bash CreateFileMapping / Win32 error 5; corrected direct PowerShell invocations produced the results above.
Prerequisite merge: replace-cfgrules28 merged as requested. engine/CMakeLists.txt conflicted; automatic -X ours kept current content. Coordinator must reconcile the incoming build registration. No engine source was inspected or manually edited.
No REA call was needed; evidence came from re_query.py, existing decomp ranges, and read-only executable constants. No proposed ABI-row file, C ABI export change authored, game run, or shadow claim.
