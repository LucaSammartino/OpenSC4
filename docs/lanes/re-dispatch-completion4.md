| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-dispatch-completion4: native dispatch contracts, final round 3/3 | `re-dispatch-completion4` / (RE lane, detached worktree) | PARK/WIP: DP11–DP13 answered previously; substantive DP10 texture/rectangle/draw contracts committed; exact rendering residual remains; docs checks pass | Coordinator review partial contracts; preserve worktree; no fourth round or replacement credit. |

## Final report (2026-10-22, round 3/3)

Coordinator publication review (2026-10-22): accept the already committed bounded DP10 rendering contracts and prior DP11-DP13 answers for integration, after reviewing the precise remaining rendering boundary and clean tree. DP10 stays parked and incomplete. No new RE, replacement/default credit, fourth round or worktree removal is authorized.

DP10 remains **PARKED**, with exact residual addresses in dispatch §4.1.3 and its open
question. This final round published substantive native contracts rather than claiming
a complete view provider. DP11, DP12 and DP13 were already answered; their vehicle,
pylon and air-source proofs were not repeated. No fourth round is authorized.

### Published this round

- b6914951: call-number texture rebuild, native font/surface slots and argument order,
  style save/restore and ownership, destructive replacement, failure exits and
  distinguishing size/failure vectors. First substantive contract was committed
  within the requested call-20–30 window.
- e282002b: borrowed GetLastScreenIconRect return/layout, zero/reversed extents,
  integer rectangle/float anchor stores, stage/context/render-state calls, exact
  final 2D submission order, handle-pointer identity and null-texture behavior.
- 5db61bde: exact conflict-helper layout/return channel, category filtering,
  inclusive bounds/closed-edge overlaps, first-conflict behavior, four ordered
  probes, travel limit and overlapping fallback.
- e27f9d75: verified intermediate icon/number quads, UV dimensions/rounding,
  worked example, progress's x87 comparison, and precise final PARK residual.

All four milestones update dispatch §4.1.3. Independent vectors are prose/native
requirements, not claims of executed tests or fits to replacement output.

### Evidence and measured constants

Queried the configured export first with main-checkout re_query.py, read the native
cSC4DispatchVehicleView decompilation by bounded ranges, and verified numeric/call
details with instruction bytes. Main bodies: BuildCallNumberDisplay 0046C8B0
(2112 bytes), Update 0046D990 (5618 bytes), rectangle getter 0046BFA0 (66 bytes),
conflict helper 0046B830 (192 bytes), power-of-two helper 006046B0 (23 bytes),
viewport conversion 007F6690 (13 bytes). No decompilation or assembly was copied
into specs; citations identify original addresses.

Read-only PE measurements independently confirm `%d`, float32 0/0.5/1,
zoom-offset table 20/30/40/50/60/60, icon size42, center offset21,
probe spacing43, travel multiplier1.75 and step2. Category bytes for kinds0..6
are 1/1/1/1/2/2/3. The ignored helper is build-runs/dispatch_pe_read.py;
no game bytes or generated RE output are committed. REA was not needed/invoked.
q.py could not start Bash (E_ACCESSDENIED); bounded PowerShell reads worked.

### Exact parked residual

- 0046CF41–0046D0DB: camera rotation's final coefficients/vertex operation order.
- 0046CB52–0046CCDA: kinds3/4 resource generation/geometry/failure branches.
- 0046DCBA–0046E296: complete visible-entry eligibility/priority, occupant/terrain
  and projected-point preparation before rectangle placement.
- 0046EA4A–0046EDF1: final 2D icon/number vertex construction/rotation; submission
  order itself is already specified.
- 0046E845–0046E9DA and 0046DE07: alternate world-space two-pass/3D-model paths.

DP10 stays in docs/specs/questions/dispatch.md, narrowed to this residual; no false
ANSWERED entry was archived. DP11–DP13 remain archived from prior rounds.
questions.py index completed; its generated index had no resulting tracked change.
No new question IDs or ABI proposals. No docs/abi tables or C ABI exports changed.

### Validation and promotion limits

git diff 7e093616 --check passes. Round-3 scope is three public documentation files:
dispatch spec, its open-question file and this report. Cargo/clippy/ctest are n/a
for spec text; no engine, tools, maps or implementation/test outputs were read or
edited as contract. No runtime trace/game run was requested or performed.

Public glue-dispatch-complete4 measurements remain historical gap evidence only:
original a3 clean/ref0, shadow1694/1696 values exact, with two unsaved view handles
differing; replacement a1 incomplete. This round does not claim those gaps were
remeasured or fixed. No game request is outstanding.

No complete view-provider, complete-owner, supported replacement, exact-shadow,
default or runtime promotion. Preserve this reserved detached worktree and its
commits. No fourth round, subagents, self-restart, external communications/network,
merge, push or ref/config repair. Detached commits succeeded despite Git's
packed-refs.lock permission diagnostic. Current guard manifest:
C:/Users/tlori/FreeCity4/build-runs/run4-re-dispatch-completion4-r3-manifest.json.

Coordinator next action: review and retain the partial native contracts and WIP
queue record; keep DP10 parked and unsupported callbacks unpromoted.
