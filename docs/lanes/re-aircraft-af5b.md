| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-aircraft-af5b (aircraft flight contracts) | Codex / detached `re-aircraft-af5b` | Ready partial spec increment; AF2–AF6 remain open; doccheck passed; ABI unchanged | Complete AF2 attitude/environment/rotor model, then AF3–AF6; owners remain unclosed |

## Report

The requested AF2–AF6 answers and aircraft owner closure are **not complete**.
This documentation increment is ready to land, but neither airplane nor helicopter
can be treated as closed from it. No question was marked answered or archived.

### Changed

- Aircraft §3.4.1 now specifies the physics seconds conversion and cap, the
  whole-velocity limiter (including promotion of tiny nonzero velocity to speed
  1), candidate integration using mean velocity, and correction ordering.
- Aircraft §3.4.2 specifies the scalar force model: vertical-component angle
  expression, inverse cosine, lift interpolation and its equality branch, flap
  scaling, drag normalization, gravity, rolling resistance and reverse-throttle
  gate. It explicitly leaves the attitude-derived lift direction open.
- Added six conformance vectors, numbered 8–13, including a stall-angle endpoint
  case that distinguishes the interpolation branches.
- Updated existing AF2 with the exact remaining scope. AF3–AF6 are unchanged.
  Regenerated the question index; its tracked contents did not change.

### Evidence and limits

Queried the main export first and read selected local airplane pseudocode.
`UpdatePhysics` has no local pseudocode; its candidate motion, limiter and ordering
were reconstructed from instruction bytes. Numeric constants were read from the
original PE using an ignored, read-only helper in this worktree's build-runs.
The spec includes short address citations rather than source listings.

The animation pseudocode incorrectly appears to pass milliseconds to physics;
the instructions reload the converted seconds local. The export's misleading
`TransformDirection` name at `0x009F0090` was checked against its math body: it is
inverse cosine. Those source discrepancies are recorded in the owning spec.

Research and numerical wording review stopped at **93,828 / 150,000** measured
context, below the required two-thirds ceiling of 100,000. The remaining budget
is reserved for documentation validation and reporting. No REA request was made
this round; the predecessor reported an unavailable target, and this increment
uses the export/decomp fallback. No new tools or sessions were configured.

The prompt both prohibited engine reads and requested an engine README read.
The stricter read boundary was retained: the core-aircraft5 lane report supplied
the unsupported scope. No engine, plugin, capi, coverage-map or ABI files changed;
there are no proposed ABI rows.

### Validation

- `python tools/doccheck.py check`: **PASS**, `doccheck check: ok`.
- `python tools/questions.py check`: **PASS**, `questions: ok`.
- `python tools/questions.py index`: completed; no tracked index delta.
- `git diff --check`: **PASS**.
- Cargo, clippy, CTest and game runs: **n/a**, documentation-only increment.
- No outstanding game labels; no C ABI changes.
- Commits: `abd8be90` initial handoff, `98ee9096` numerical contracts,
  `1c7e6d6e` boundary precision. Detached HEAD retained. Git reports a denied
  packed-refs lock attempt while committing, but the commits succeed; shared
  refs/config were not modified or repaired.

## Handoff

- Goal: answer AF2–AF6 in aircraft.md with instruction-verified numerical rules.
- Done: `98ee9096`, `1c7e6d6e` specify partial AF2 and vectors 8–13; AF1/AF7/AF8 were answered before this lane.
- Next exact step: reconstruct airplane angular/control helper `0x004FE050` (2043 bytes; missing local pseudocode), then complete CalculateForces lift-frame derivation.
- Then finish UpdatePhysics environment branches `0x00503E84–0x0050431D`, collision rules and helicopter rotor model.
- AF2–AF6 stay open; owners remain unclosed. Do not archive their partial progress as answers.
- Force curve equality uses the lower segment; tiny-speed comparison uses x87 result but maximum comparison uses saved binary32 speed.
- Caller passes converted seconds; export helper `0x009F0090` is acos, despite its name.
- Main RE sources: C:/Users/tlori/FreeCity4/decomp/cSC4Airplane.c and cSC4Helicopter.c.
- Plane CalculateForces local line 724; helicopter NextFrame 2113; prior handoff has other entry offsets.
- No engine reads/edits, REA requests, ABI changes, game requests or ABI-row proposals this round.
- No milestone below means the remaining AF3–AF6 research was completed; report is ready only as a spec increment.
