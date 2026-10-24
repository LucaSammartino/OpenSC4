| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-effects-capture8: retained visual records | Codex / glue-effects-capture8 (detached HEAD) | ready: effects 73/73 exact, zero resyncs, ref_warnings=0; cargo 6922/0, Clippy clean, CTest 74/74; EF21 open | Land measured scope; answer EF21 before broader capture; coordinator handles profile and generated index. |

## Report

Extended default-off effects_shadow from name membership to retained visual
record comparison through engine/core/src/effects/capture.rs. Native effects
remain active. The shadow independently decodes the raw directory resource,
then compares the manager collection's retained visual records. It never
repairs or resynchronizes from native outputs. Replacement-mode mirroring
retains its existing separate path.

The bounded reader implements EF20 vector/string/hash framing, logical ends,
stable indices, duplicate names, dictionary count validation, global node
cycle/repetition detection and unreadable-span refusal. ReadProcessMemory
copies values into capture-owned memory; no description pointers escape.
Compared: visual flags/priority/extra words/text; component names, types, flags,
numeric transform words/revision/defined transform flags, LOD/copies/spacing/
render/select/index; parameter-binding names/flags/float bits/index; complete
visual (name,index) multiset. Float comparisons preserve signed zero and NaN
payloads. Dictionary bucket order, pointer/capacity words, padding and
transform flag bits outside the three load-defined bits are excluded.
EF19's independent legacy-to-memory transform mapping is used.

No collection getters are called: EF20 says even null-output lookups mutate
scratch state and value outputs require constructed owned destinations.
Resource slot 5 is never called for capture: it publishes and can exchange
native storage. Existing SDK raw-record cleanup and manager membership queries
remain in use. The memory path acquires no references and calls no owner methods.
Native resource identity discovery, natural publication/bootstrap tracing,
remaining family layouts and confirmed quiescence/attachment contracts are
tracked by **EF21** in [questions/effects.md](../specs/questions/effects.md).
Queue inputs and historical bootstrap choices cannot be independently recovered
from a resource decode. Those paths remain unimplemented and disabled.
Camera copies, source ID/message bindings and derived tables are also outside
this independent-input comparison; parameter bindings inside visual records
are compared.

Added C ABI export fc4_effects_verify_visuals; inherited
fc4_effects_verify_names from shadow7. ABI version unchanged. Core focused
tests **3/3** cover exact payloads, duplicate names, spare capacity, padding,
NaN bits, signed zero, malformed vectors, cycles, out-of-range/repeated nodes
and missing records; the ABI test additionally checks refusal handling.
No generated ABI entries, game files, coverage maps or tools were edited.

## Exact coverage credit boundary

**Wholly compared effects coverage-map rows: none.** This ready partial
measurement expands a visual-record subset, not any whole owner. The table
uses the brief's owners; forbidden tools coverage tables were not read.

| Effects owner row | Wholly compared? | Measured or missing scope |
|---|---|---|
| cSC4EffectsManager | No | Membership probes and retained visual collection records; live effects, lifecycle, derived publication tables and cameras omitted. |
| cSC4EffectsResource | No | Raw directory is independent input; no live resource payload, stream lifecycle or publication-history comparison. |
| cSC4EffectsCollection | No | Visual payload and duplicate name/index subset only; other families, anonymous descriptions, publication/fixups, camera, ID bindings and triggers omitted. |
| cSC4EffectsParser / EffectsParser | No | No retained queue/diagnostic/modified-state or natural parse trace comparison. |
| EffectsBootstrap | No | No natural service/resource/parse/message ordering trace. |
| cSC4BrushDescription / cSC4ScrubberDescription | No | Referenced component indices compared; description payloads omitted. |
| Other effects rows | No | No whole-row equivalence claimed. |

## Validation and game measurement

Stable engine source **46c67c95c**, unchanged throughout gate and game:

- qbuild all **PASS**: Cargo **6922 passed, 0 failed, 0 ignored**, summed
  over all result lines; Clippy **0 diagnostics**, -D warnings.
- Queued native operation **qbuild-20261024-052549-build-ctest**, **PASS**:
  build **240/240** (107.4 s), CTest **74/74** (146.2 s), overall **257.2 s**.
  Native verification went through the coordinator queue; the same operation
  was collected to termination. No direct MSVC build or duplicate verify.
- Explicit fmt **4 files, 0 diff lines**; git diff --check clean. Logs:
  build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log,
  fc4verify-ctest.log. Earlier Bash-wrapper and relative fmt-path errors
  were corrected transport diagnostics, not source gate failures.

Final counted line: **effects8-visuals-2032-20261024a 73/73**

[Run 412](../runs/effects8-visuals-2032-20261024a-412.md): **clean**,
ref_warnings=0, all problem counts zero. All **73** load/month samples have
mismatches=0, refusal=0, resyncs=0, from **2026-01-27 load** through
**2032-01-01 month**. Each sample reads **1,094,484 resource bytes**, compares
**1,154 visual records**, performs **108,706 typed value checks** and
**3,465 native name probes** over **1,154 raw name entries**. Totals:
**7,935,538 typed checks**, **252,945 probes**. Checks include aggregate/string
comparisons; they do not imply other-family equivalence. Harness reports
"effects 73 lines, 73 with every mismatch count 0", "effects resynced lines: 0".
Turbo shutdown date **2032-01-11**, city runtime **35.500 s**, wall **49 s**,
slot **3**. DLL SHA256:
662E782A424E1E1B2701AB9451571F2491A4EFFBB7265AEFFF7F8770BDFCDA6C.
No turbo-only difference, so no normal-speed rerun was required.

One unique request, one blocking filesystem-event waiter without qwait; its
same session was collected to termination. No build/game request outstanding.
Evidence: build-runs/game-results/; build-runs/measure-effects.py reproduces
the counted line.

## Merge and next steps

Requested octopus merge **52edd0f0e** omitted both branches' changes after
a sandbox Git helper failure. **165a86708** restores the requested committed
engine/spec/report trees. EF20 is already absent from merged open questions.
No forbidden unrelated lane or main merge. Detached commits succeed despite
the shared packed-refs.lock housekeeping warning; shared refs were not changed.

Ready queue entry is for this measured subset. Supervisor publishes detached
HEAD to the reserved lane branch. Coordinator regenerates the questions index
for EF21 and reviews scope before crediting coverage. The shadows-profile
append requires coordinator follow-up on current main: clean-room restrictions
prohibit this lane from reading/editing tools/regress.ps1. Defaults remain off.
Answer EF21 before broader payload/queue/bootstrap capture or whole-owner credit.
