| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-grid-interpolation4: byte-grid interpolation contracts | Codex / re-grid-interpolation4 | ready; docs only, ordinary algorithm supported, boundary questions partial | Implement from spec; resolve GINT1–GINT3 only with further authorized evidence. |

## Report

Published [grid-interpolation.md](../specs/grid-interpolation.md) for precisely
the two attributed bodies, `0079EB40` (587 bytes) and `0079EFD0` (593 bytes).
Initial admission/handoff is d939fc29; substantive algorithm publication is
4d20d394. The detached worktree was reused; no refs were moved by this worker.
Copied coordinator admission and current re/round-1 manifest were consumed;
source base was ec52575cee183bf015a243ef4fc9781ff9673b7f. No prior dedicated
logical family was discovered in the scoped evidence.

The original file version is 1.1.641.0; SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`
matches the Windows export summary. Export queries and byte-bearing instructions
covered both complete contiguous bodies, their sole direct callsites
(`007A2489`, `007A2840`), and necessary descriptor preparation at `007A1310`.
There are no direct callees or entry data xrefs for either helper. They are
distinct bodies, with no attributed folded/shared additional entry. Local
decompilation was read only for supporting signature context. REA was not used:
the canonical export/original evidence was sufficient; no live readiness or
native execution is claimed.

Supported ordinary behavior: caller-owned nested byte storage with 12-byte
outer descriptors; quantized 16.16 centre coordinates; clamped edges;
inner-axis rounding before outer-axis rounding; upward ties; unsigned loads
versus signed loads followed by final byte bias 128. There is no allocation,
zero-fill boundary default, saturation, floating-point dependency, or host
whole-interpolation delegation in either owned body.

Twelve independent literal vector cases cover nonuniform 2×2/3×3 inputs,
asymmetric axes, enlargement/reduction, quantized thirds, equality, signed
negative/zero/high values, bias boundaries, rounding order, ties and valid
singleton axes. Every vector is proposed and manually derived from native
instruction constants; none was executed against the original or an engine.
For example, the asymmetric case gives 127 where exact-half mapping gives 128;
the two-stage rounding case gives 1 where one final rounding gives 0.

Partial/unknown: caller allocator failure semantics, foreign city dimension
guarantees, large unchecked dimensions, malformed storage and general overlap.
The caller constructs first-target-count descriptors but sizes
second-target-count elements, so unequal target counts do not establish safe
rectangular storage. This does not establish reachability of rectangular cities.
Filed [GINT1–GINT3](../specs/questions/grid-interpolation.md); no preexisting
question was moved or marked answered. QV8's ordinary resampling algorithm is
now specified here, but palette generation and all other MapView public,
rendering, lifetime and query gaps remain open. No coverage/default promotion.

Validation: question index regeneration and `questions.py check` pass; diff
whitespace and local link targets checked. Vector arithmetic was manually
reviewed. Cargo/clippy/ctest: n/a, documentation-only. No builds, tests, game
runs, native probes, process/RPC/network actions, or engine/test reads. Game
labels and shadow verdicts: n/a. No C ABI changes or ABI row proposals.

Next: implement the ordinary contract using the proposed literals as review
inputs, with no native-pass claim. Further native boundary work requires a
separate authorized task; this invocation does not restart another family/round.
