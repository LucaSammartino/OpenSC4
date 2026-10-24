| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-effects-whole9: EF21 bounded capture and safe measurement | Codex / glue-effects-whole9 (detached HEAD) | ready partial: names 73/73 exact, 0 resyncs, foreign_exceptions=0, ref_warnings=0; Rust 6954/0, Clippy clean, CTest 78/78; EF22/EF23 open; no whole-owner credit | (1) Answer EF22/EF23; (2) attach natural observers and close all owner scope; (3) coordinator reconciles coverage/index/profile. |

## Report

**Ready measured partial work; the whole-manager/resource objective is not
achieved. No effects owner receives whole-row comparison credit.**

7050b5688 merges the requested `re-effects-ef21` answer, preserving EF20 and
removing the stale open EF21 entry. 49cbc7e5e adds the bounded EF21 reader,
observation bookkeeping, thirteen conformance fixtures and safe plugin gating;
3dd46ce91 validates/documents the fixtures; 88180deb2 fixes the gate diagnostic.
Engine source remained at **88180deb2** throughout final verification and game.

### Implemented scope

`engine/core/src/effects/capture/payload.rs` copies every EF21-listed built-in
payload coordinate: particle, dynamic/model, decal, shake, light and component
types 3 through 8. It recursively copies nested vectors and byte strings,
preserves exact float bits, vector order, duplicate IDs, anonymous slots,
trace-local alias identities and family name/index framing. It excludes vptrs,
reference counts and specified padding. Particle +1C8 is explicitly marked
indeterminate rather than assigned an invented default. Nonempty reserved
type-9..15 slots preserve framing without dereferencing foreign payloads and
make support false. Sequence vectors are copied, but complete sequence payload
support stays false pending EF23. The reader now has a **16 MiB aggregate byte
budget**, checked addresses, bounded vectors/strings and refusal on malformed
or unreadable spans. These owned copies are evidence, not independent goldens.

`capture/observation.rs` supplies tested host bookkeeping for thread identity,
outer operation depth, Init completion, fresh tick generations, original
registration flags, ordered parse/diagnostic/message events, request failures,
cache hits, repeated borrowed resource leases and pre/post publication markers.
Late attachment never makes bootstrap history available. A success message or
End return inside reload does not permit a combined snapshot. Writer exclusion
remains an explicit host precondition; bookkeeping cannot prove unobserved
native writers absent. No native interception is installed by this lane.

Thirteen executable **synthetic** EF21-1..13 fixtures live in
`engine/conformance/vectors/ef21_*.json`, with runner operations documented in
`engine/conformance/README.md`. They test bookkeeping and copied payload values,
not native bootstrap, publication execution, stream deserialization or live
effect equivalence. Core tests additionally check malformed/null/wrapping
addresses, total read budget and repeated/released leases.

### Exact coverage credit boundary

EF21 section 18.8.2 invalidates capture8's assumption that load/month delivery
alone proves quiescence. `FC4EffectsShadow.cpp` therefore stops private visual
traversal at those callbacks. The default-off key still measures independent
raw-directory decoding against the SDK's native HasVisualEffect method. Every
sample explicitly logs `payload_unavailable=1 history_unavailable=1`, with
`visuals=0 fields=0`. **The old visual-payload credit is not carried forward.**
Zero name mismatches must never be promoted to payload or whole-owner credit.

The following uses the owner labels supplied in the lane brief. Forbidden
tools coverage tables were neither read nor edited; the coordinator reconciles
actual map rows. **Wholly compared effects rows: none.**

| Effects owner row | Wholly compared? | Measured / missing scope |
|---|---|---|
| cSC4EffectsManager | No | Name membership measured; natural Init/reload history, private retained/live/new/active state, lifecycle, cameras, derived ID/trigger tables and method outcomes unmeasured. |
| cSC4EffectsResource | No | Raw directory is independent input; borrowed native resource attachment, stream/reference/serialization history and pre/post publication unmeasured. |
| cSC4EffectsCollection | No | Bounded payload copying tested synthetically; native payload, publication/fixups, bindings, triggers and cameras unmeasured. |
| cSC4EffectsParser / EffectsParser | No | Ordered bookkeeping tested; natural queue, flags, diagnostics, modified state and parse trace unmeasured. |
| EffectsBootstrap | No | Synthetic request/publication lifetime tests; natural service/options/resource/parse/message history unavailable. |
| cSC4BrushDescription / cSC4ScrubberDescription | No | EF21-listed payload fields copied in synthetic memory; no native full-owner measurement. |
| Other effects rows | No | No whole-row equivalence claimed. |

### Validation and game measurement

Focused core capture **6/6 PASS** and EF21 conformance **13/13 PASS**. Explicit
fmt checked six touched Rust files with zero diff lines; the final changed file
was rechecked after its Clippy fix. `git diff --check` clean. No C ABI export,
ABI version or generated ABI entry changes.

Final `qbuild all` on stable engine source **88180deb2: PASS**:

- Cargo **6954 passed, 0 failed, 0 ignored**, summed over every result line.
- Clippy **0 diagnostics**, `-D warnings`.
- Coordinator-queued native operation **qbuild-20261024-062533-build-ctest**:
  build **47/47** (74.8 s), CTest **78/78** (146.0 s), overall **222.5 s**.
  The tree-change notice is solely documentation commit 971a93a53 during the
  operation; no engine source changed. Both native operations were collected
  to termination; no duplicate verify or direct MSVC build was launched.

First full gate on 3dd46ce91 passed Rust 6954/0 and native build **249/249** /
CTest **78/78**, but failed Clippy's collapsible_match check. Its evidence is
preserved in `build-runs/failed-3dd46ce91-clippy.log`; 88180deb2 applies the
recommended match guard. Earlier failed core expectation, Json::String compile
diagnostic and Bash transport errors remain in unique `build-runs/q/` logs.
Only this real diagnostic/source change caused the repeated full gate.

Counted line: **effects9-names-2032-20261024a 73/73**

[Run 434](../runs/effects9-names-2032-20261024a-434.md): **clean**, all problem
counts zero, **foreign_exceptions=0, ref_warnings=0**, zero resyncs. Samples run
from **2026-01-27 load** through **2032-01-01 month**. Each reads **1,094,484**
resource bytes and checks **1,154** directory name entries through **3,465**
native membership queries. Totals: **252,945 queries**, **84,242 entries**.
All 73 lines have mismatches=0, refusal=0 and resyncs=0. Payload/history remain
unavailable on all 73, so this exact count is **names only**. Harness confirms
"effects 73 lines, 73 with every mismatch count 0" and zero resynced lines.
Shutdown **2032-01-04**, runtime **31.875 s**, wall **45 s**, slot **2**.
DLL SHA256: **187A37ADAB22C39340EB8FB85561163B86A9F9E0DE76697344D8E5BCE96145F2**.
No turbo-only differences; no normal-speed rerun required.

One unique game request and one blocking filesystem-event waiter, without
qwait or polling loops; the same operation was collected to completion. Evidence
is in `build-runs/game-results/`; `build-runs/measure-effects.py` reproduces the
counted line. No build or game request remains outstanding.

### Questions and landing

**EF22**, [questions/effects.md](../specs/questions/effects.md), requests the
safe before-Init interceptor installation/removal contract: validated patch
spans/expected bytes, relocations, exact virtual thunk adjustments, continuation
and exception behavior, and exclusion of every writer. Observation site
addresses alone do not safely supply executable patch spans. No executable
inspection or guessed hooks were used.

**EF23** asks whether sequence flags extend EF21's stated 0x18 payload and at
which offset/mask. The vector-only capture does not claim complete sequence
payload support.

Ready queue entry is for this **measured partial work**, as the task permits
when a missing contract blocks part of the goal. Next: answer EF22/EF23, attach
before natural Init, compare real resource stream/reference/pre/post publication
history, complete independent family decoding, and measure all manager live,
camera, binding/trigger, lifecycle and method-observable scope. Re-run to 2032
before any whole-owner credit. Coordinator regenerates the question index and
reviews coverage. Any current-main shadows-profile append is coordinator work:
this lane cannot read/edit forbidden tools/regress.ps1. Defaults remain off.
