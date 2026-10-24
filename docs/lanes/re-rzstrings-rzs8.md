| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-rzstrings-rzs8: native string helper contracts | Codex / `re-rzstrings-rzs8` (RE) | ready: RZS8–RZS11 answered; 15 new vectors and RZ13 corrected; questions/doccheck passed; no game run or C ABI changes | Land spec; reconcile incoming lane's open entries; implement and measure excluded paths. |

## Report

Specification milestone: `731b9a784`.

- RZS8: §2.1 defines x86 thiscall helper boundaries, hidden result storage and callee cleanup, native construction and destruction obligations. Mid/Left/Right return the result address in EAX; MakeLower/MakeUpper are void.
- RZS9: §4 specifies exact -1/+1 unequal-prefix results in both overloads and sensitivity modes. Equal prefixes still return the original length difference.
- RZS10: §5.1 confirms field zeros before the alternate hexadecimal prefix; unsigned 3473 with `%#010x` yields `000000xd91`. Precision zeros follow the prefix.
- RZS11: §4 corrects the prior empty-needle rule to start strictly less than length. Find fails at the counted end, including an empty haystack; RFind keeps min(start,length).
- Added RZ51–RZ65 and corrected RZ13. All new expectations are static instruction deductions, not new execution measurements.
- Full source question entries and dated answers are preserved in `docs/specs/questions/answered/rz-strings.md`; regenerated `docs/specs/QUESTIONS.md`.

Evidence: queried the main checkout's export first with re_query.py, then read bounded cRZString decomp sections and verified numeric widths, stack cleanup, return values and ordering with asm --bytes. Main evidence addresses are cited in the spec. REA was neither needed nor called.

Validation: `python tools/questions.py index`, `python tools/questions.py check`, `python tools/doccheck.py check`, and `git diff --check` passed. Cargo/clippy/CTest and game/shadow runs are n/a for this documentation change. No engine, plugin, capi, coverage-map, ABI tables or C exports changed; no proposed ABI rows.

Integration: RZS8–RZS11 and the originating lane report exist only in the glue-rzstrings-shadow5 worktree, not this base's or main's open-question file. Their complete entries were therefore archived directly; no local open entry could be removed. If the originating lane lands its question file, remove those four open entries and regenerate the index using these answers. The lane report and questions supplied the four blocked exclusions; engine/ was not read because this task explicitly forbids it, including its Owner closure table. No coverage credit is claimed.

Tool notes: q.py failed to launch Git Bash (CreateFileMapping access error), so bounded local PowerShell commands were used. The first commit emitted a packed-refs.lock permission diagnostic but successfully created detached commit `731b9a784`; no ref/config/maintenance operation was attempted.

## Handoff

- Goal: close RZS8–RZS11 for glue-rzstrings-shadow5 using original export evidence.
- Worktree is detached at reserved base; do not switch branches.
- Done: `731b9a784` settles all four answers; fifteen new vectors, RZ13 corrected; questions/doccheck passed.
- Next: coordinator lands docs, reconciles incoming open entries if necessary, then resumes blocked implementation/shadow work.
- No game-request labels outstanding; no native execution or new shadow coverage claimed.
