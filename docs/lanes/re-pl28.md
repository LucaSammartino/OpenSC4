| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-pl28: PL24 lot-prop bounds; kind docs | Codex / .claude/worktrees/re-pl28 / reserved re-pl28 (detached HEAD) | ready; prerequisite merge 1877063ab, spec 04a57add4; ABI unchanged, no seam reservations; scoped docs checks pass; game n/a, no shadow credit; PL24 answered | 1. Land docs. 2. Engine lane implements closed upper bounds and reruns propmgr verification. |

- Merged propmgr-query27 as requested; its inherited engine changes were not read or independently edited.
- Authored changes are spec text, the PL24 archive move and regenerated question index.
- Native slot 24 uses closed world bounds on both axes: wx1 ≤ px ≤ wx2, wz1 ≤ pz ≤ wz2.
- Lower X, lower Z, upper X, upper Z are tested in that order; any unordered comparison rejects. Y is ignored.
- Comparisons use the supplied binary32 positions/bounds directly; no tolerance and no GetPropType=0 exception.
- Evidence: 0x004A3240, especially lower tests 0x004A3316–0x004A3332 and upper tests 0x004A3334–0x004A3350.
- prop-lotconfig.md A7.2 and T.4 now specify equality, adjacent binary32 values, NaNs, caller entries and stable order.
- PL24 and both sources of evidence moved to docs/specs/questions/answered/prop-lotconfig.md, marked ANSWERED 2026-10-26 (re-pl28): DIFFERS.
- Validation passed: focused prop-lotconfig question check, full generated-index freshness, PL24 archive presence/open absence, git diff --check.
- Global questions.py check still reports the pre-existing NT36 open/archive conflict in network-tools; those files are unchanged from base 73aa6a7fd.
- Cargo/clippy/ctest and game: n/a for this spec task. No new game run, mismatch-fix claim or verified credit.
- REA was not needed. No ABI exports or proposed ABI rows changed.
- Next: apply this contract in the engine lane, test boundary fixtures, then obtain fresh propmgr shadow results.
