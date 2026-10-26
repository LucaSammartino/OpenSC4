| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-le30 lot editor LE9–LE18; kind docs | Codex / re-le30 / reserved re-le30, detached HEAD | continue; commits 7230661cb, 082324e5a, a2c401b56, c67bc7f69; ABI false; no game runs; no shadow measurement; LE9–LE18 archived; doccheck/questions check not run | 1. Apply the two checked corrections below. 2. Review bounded runtime answers and reconcile stale cross-references. 3. Run checks directly in PowerShell, commit, then finish and queue kind docs. |

## Handoff
Goal: answer or narrowly bound LE9–LE18, provide worked cases and README-row mappings, pass doccheck and enqueue docs; never read/edit engine/.
Done: behavior spec §§27–36; all ten original entries moved to the named answered archive with dated markers; questions index regenerated after each milestone.
7230661cb: LE9/10 hit, rectangle and masked-x87 conversion rules. 082324e5a: LE11/12 wall/foundation selectors and sizing.
a2c401b56: LE13/14 popup/wrappers, edge table, shared primary-grid sorting/search and names. c67bc7f69: LE15–18 resources, input, partial rendering and attribution boundaries.
Next: a patch immediately before guard denial did NOT run. In §31.1 change texture cursor field from +0x158 to +0x154; main decomp lines 6535–6540 proves this.
Also correct §35 foundation name helper: successful nonempty name gets parenthesized description; no initial output clear, failed acquisition appends Unknown implementation type without delimiter. Property 0x68FCFF37 present -> type 3, absent -> type 1; type-2 Generic label exists but this route never produces 2.
Source for that correction: re_query asm 0x0047FF30 --bytes --skip 10 --max 56, especially 0x0047FFA3..0x0047FFC1. Main decomp lines 3294–3361 shows formatting and missing-clear behavior.
Checks: q.py doccheck and questions check both failed to launch Bash with CreateFileMapping Win32 error 5; neither Python check executed. Rerun directly using absolute WT/tools/doccheck.py check and WT/tools/questions.py check. Do not call those failed checks passed.
git diff --check HEAD~4 HEAD completed with no output after the wrapper failures; rerun for final edits. Git commits succeeded despite a packed-refs.lock permission warning from git add.
Important: LE17 is narrowed to LE17-R1–R4 call/buffer/terrain/resource observations, not full static closure; §35 maps the rows still partial. LE16-R is an actual list-end read; no deterministic heap outcome. LE18 exclusive source ownership cannot be proved by absence of runtime hits and needs an unavailable provenance map.
No REA calls, no ABI rows, no original/decompiler files committed, no engine/ access, no merge queue written. Helpers/messages are gitignored in WT/build-runs/.
Main absolute root C:/Users/tlori/FreeCity4; source helper WT/build-runs/read-le30.py reads <=80-line ranges and original PE data, UTF-8 stdout. All original sources read from main.
Primary grid table is 0x00ADD578: +0x20C=0x009ACED6; +0x214=0x009A9892. The exported 0x00ADD7B0 is a different window subobject. re_query --full also needs --limit raised when filtering more than 40 vtable rows.
_ftol2 0x009EEF04 converts through signed64; masked-invalid indefinite has low32 zero. Forward hit scan top alone spills reciprocal to binary32. SetLotSize's aspect-dependent branch is intentionally unusual.
Closure references are by method/address because engine README access is forbidden. Historical inventory counts were not changed. Review stale §3/§12/§18 stopping text against new §§27–36 before finalizing.
Next: finish corrections and verification in a fresh context, then write final lane report/queue using actual full HEAD and reserved branch re-le30.
