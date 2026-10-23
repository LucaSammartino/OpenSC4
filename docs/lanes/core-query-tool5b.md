| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-query-tool5b (query owner follow-up) | Codex / detached core-query-tool5b | blocked on native gate; core implementation committed; cargo 6091/0, clippy clean, fmt clean; CMake compiler-check link stalled, CTest not reached | Resolve native build environment, rerun full gate after changed inputs; review CT21; retain residual owner questions |

## Report

Implemented the settled query-owner supplements in `engine/core/src/query_tool`.
No plugin/capi wiring, shared base duplication, ABI changes, generated ABI edits,
coverage-map edits, branch/ref changes or game requests. Commits: ab95d746
(implementation/tests), bfb4006c (closure audit/CT21), a7623092 (gate checkpoint).

Changes:
- All six funding related-ID lists, including hospital staff's literal integer IDs
  `0x1023`, `0x1024`, `4`, `0x3F800000`; confirmed null occupant landfill slots and separate zone dispatch.
- Best-developer selection: all zone candidate orders, developed-record path,
  signed16 desirability, truncated demand tie-break, stable ties and sentinel.
- Subjective rating fillers use selected developer curves and thresholds. Commute
  handles unsigned bytes/zero substitution, truncated comparisons and auxiliary
  values, absent curve medium, and commute-specific labels.
- Wealth meter uses binary32 `0x3EAAAAAB`, independent occupant/building wealth,
  right alignment and retained window reference-left property across refreshes.
- Constructed multipoint curves clamp endpoints and use stored binary32 slopes
  with widened intermediates. Property parsing/slope construction remains open.
- Airport lost luggage always uses the complete fixed resource key; failed lookup
  returns failure, absent curve property gives the specified default.
- Neighbour plans implement last-successful network priority and all one/two-row
  scripts. CT14 row fillers still prevent opening a faithful neighbour dialog.
- Live debug enable combines the application setting with Shift/Ctrl/Alt. Known
  bookkeeping and cell-writer order is traced explicitly; no writer body is
  delegated to a native query-owner callback or replaced with invented text.
- Rechecked every existing README owner row and added every §8.16 subsection.
  The audit also repaired DescribePick's missing terrain-hit lot-building fallback.

Validation:
- Focused query suite: 32/32 passed (20 existing tests plus 12 supplement tests
  covering QCT5-1..16's settled primitive contracts). Initial 30/32 fixture failures
  were corrected; evidence: `build-runs/query-focused-first-failure.txt`.
- Explicit owned-source `qbuild fmt`: 6 files, diff_lines=0. Formatting fixes were
  applied through apply_patch, never by writing rustfmt stdout into source.
- One final `qbuild all` on stable source bfb4006c: workspace cargo **6091 passed,
  0 failed**, summed over every result line; Clippy **clean with -D warnings**.
- Native gate **incomplete**: CMake stalled at the initial C compiler ABI
  try-compile link for over ten minutes. The compiler object existed but no
  executable appeared; no CMake build or CTest result is claimed. Collected the
  original operation (session 43173) to terminal exit 1 by interrupting it after
  the measured stall; no duplicate check or shared-process kill.
- Evidence: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-configure.log`, `query-final-gate-interrupted.txt`, and
  `engine/build-x86/CMakeFiles/CMakeScratch/TryCompile-17bbj5`.
- q.py failed before Cargo (Bash CreateFileMapping error 5); direct PowerShell
  focused Cargo worked. Process-details lookup via CIM was denied. Git commits
  succeeded despite the denied shared packed-refs.lock maintenance attempt.
- Game/shadow: n/a, core-only scope with no adapter integration; no outstanding request.

Questions and remaining owner boundaries:
- Filed **CT21** in `docs/specs/questions/city-tools-input.md`: §8.16.1/QCT5-3's
  extra ID+2 cost reference conflicts with §8.12.4/§8.13's explicit ID+0x100
  behavior. Retained the explicit helper/notification target and marked the
  additional update Unsupported. Coordinator must regenerate the question index;
  this engine lane did not run tools/ scripts or edit generated QUESTIONS.md.
- Retained **CT14**: §8.13 neighbour row fillers, §8.12.4 MySim, complete writers
  in §8.11.4/§8.12.1. **CT18**: §8.12.1 empty-cell sound conflict only.
- Retained **CT19**: §8.12.4 airport noise, curve property construction/malformed
  inputs, missing records, zero commercial capacity and exceptional numeric
  conversions/sums; §8.14 aura rectangle endpoints. **CT20**: §8.7/§8.11.4 traffic
  tips and trip-volume helpers, §8.12.1 full writer bodies/network backing fields.
- Earlier unresolved CT6 (external reader), CT7 (legend), CT9 (modifier action),
  CT13 (native callout mapping) and CT15 (route generation/drawing) stay open.
  Known ordering traces do not close output-text writer contracts; QCT5-16's
  complete final text is still CT14/CT20. Overall owner closure is not claimed.

Next: resolve the native configure/link environment, preserve failed evidence,
rerun the full gate only after changed inputs, then promote the wip queue entry.
Resolve CT21 and the retained spec questions before claiming full owner closure.
