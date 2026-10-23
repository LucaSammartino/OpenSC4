| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-system-service-pointer4: original compiled service pointer utility | Codex / re-system-service-pointer4 | **ready**, docs only: R1 d3a7d0aa/38e66f4d retained; R2 b00d2d44 Draw cleanup proof and 7f44a04d output preparation proof. Static doc/question checks pass; cargo/clippy/ctest n/a; game n/a; ABI unchanged. SSP1 answered, SSP2/SSP3 open. | Coordinator review and land bounded docs. Source attribution SSP2 and exceptional domain SSP3 remain deferred; no automatic third round. |

## Report — original R2 of 3, 2026-10-23

The round resumed the existing clean detached HEAD `38e66f4d253a02d10cba50b465690d529a8426c4`.
It preserved the first round's normal contracts, 43-entry classification,
eight instruction-shape groups, assignment continuation `0x004F2075`,
proposed S/A/L traces and ignored analytic evidence. No reset, branch switch,
merge, push, source-map change, ABI row or implementation change was made.

### New substantive answers

- `b00d2d44`: SSP1 is answered in spec sections 3.4/5.4. Draw construction
  at `0x0099D041` (W=`EBP-8`) and `0x0099D118` (W=`EBP-4`) is followed by
  direct calls at `0x0099D054`/`0x0099D153` to shared callable D74 cleanup
  `0x004495C0`. ECX enters as W, becomes [W] only inside cleanup, and calls
  virtual `+0x08` without clearing W. The arithmetic body `0x0082AE40`
  is not that destructor. The table separates caller guards, field predicates,
  unrelated owner fields, and the owner's AL result from destructor EAX.
  This does not establish a source-template or linker-folding identity.
- `7f44a04d`: sections 3.2.1/5.5 prove a live output-preparation use of
  clear-and-release body `0x00874DA3`. Three call slices return field addresses
  (`owner+0x54`, `owner+0x60`, `owner+0x28`) into subsequent foreign output
  arguments, with exact literal words and receivers recorded. The WinIME
  guards require a null field before this sequence; no Release is invented
  for their normal paths. SSP2 remains OPEN for source-language attribution.
- D01-D07 and R01-R05 add 12 independent **PROPOSED / UNEXECUTED** trace
  rows. Together with preserved S/A/L rows there are 32. They distinguish
  P/Q/null, status from written output, no-clear destruction from clear-before-
  release preparation, full EAX field-address return from owner AL result,
  and real receiver adjustment. They are not execution results or ownership
  claims about foreign factories, services or reference methods.

### Evidence and validation

Canonical main-checkout export queries preceded body claims. The matching x86
PE32 has base `0x00400000` and SHA256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
No decompiled body was needed in R2. REA live status remains UNKNOWN: no live
query or server action was performed. All foreign calls stay raw primitives.

- Ignored `build-runs/audit-service-pointer-r2.py` compared only new path
  slices and relevant utility bodies: Draw 124 instructions / 331 bytes;
  output preparation 65 instructions / 188 bytes; zero mismatches in both
  runs. These overlapping selections are static identity checks, not unique
  byte coverage or native passes. The previous all43 audit was not rerun.
- Preserved R1 helper `build-runs/audit-service-pointer.py` SHA256:
  `210ef459e3374d15c6ac673c22041e52f3fe21c59e0fc22b85b02e9f188e8d90`.
  New R2 helper SHA256:
  `cf0403b517eaf765c9211ccecac2d4b0af364e7c1137fa01458d85fd07f85aa1`.
  Selected query outputs are in ignored `build-runs/service-pointer-r2-evidence/`.
- `python tools/questions.py index` regenerated the index; `check` returned
  `questions: ok`. The SSP row now records 2 open / 1 answered / next SSP4.
  Bold entry formatting was corrected so these entries are actually counted.
- Ignored `build-runs/check-service-pointer-docs-r2.py` passed: 32 unique
  proposed trace rows, SSP1 answered, SSP2/SSP3 open, and changed-file scope
  restricted to five allowed documentation files. `git diff --check` passed.
- Cargo, clippy, ctest, game runs and shadow comparisons: **n/a**. No engine,
  native execution, portable oracle, build, test or ABI-export change.

### Interruption and corrected failures

R1 consumed 36 guarded calls / 74922 context and substantive commits
`d3a7d0aa`, `38e66f4d` before the actual provider error, “Selected model is
at capacity.” It was not a zero-command round or a native failure. Raw result
and stderr remain copied in `build-runs/evidence-run4/`. The actual same-model
availability probe returned exit 0 / AVAILABLE at 07:36:23; that changed input
cleared the exact capacity pause. Copied R2 admission and 07:39:42 census show
the old backend absent, no own refs/errors/services, clear write probes and
no own queue. This census is current drain evidence, not a historical cause.
No admission rerun, process action, fallback, refund or helper restart occurred.

R1's failed patch match, missing pefile (corrected with stdlib), missing
secondary decomp file and unsupported search option remain tool failures.
R2 had one failed patch match and two corrected documentation-checker defects
(syntax, then an overbroad ID pattern that included shape labels D74/D52).
Only subsequent successful runs count above. Commit commands emitted a
denied shared `packed-refs.lock` attempt but created the reported detached
commits successfully; no shared-ref repair or configuration change was made.

### Questions and next steps

SSP1 moved to `docs/specs/questions/answered/system-service-pointer.md`, marked
ANSWERED 2026-10-23 / DIFFERS, after publishing its rule in the spec.
SSP2 and SSP3 remain in `docs/specs/questions/system-service-pointer.md`:
source spelling/folding is unproved, and exceptional/reentrant/malformed,
thread and global-lifetime domains remain unresearched. No new IDs were filed.
Coordinator: review and land this bounded docs-ready result using the own
normal queue. No automatic third round is requested; any future SSP2/SSP3
investigation needs separately bounded scope. No game request is outstanding.
