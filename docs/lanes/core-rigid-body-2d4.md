| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-rigid-body-2d4: bounded portable rigid body algorithms | Codex / core-rigid-body-2d4 | ready: bounded core gate passed | Land bounded algorithms; retain RB1–RB7 and all native/default limitations |

## Result and scope

First owned portable rigid-body module in `engine/core/src/rigid_body_2d/`, with one additive Rust module export. Inspected the existing owned effects modules and searched for existing rigid-body/exact-dyadic helpers; no reusable owner or equivalent arithmetic helper was found. Effects manager, emitter, force and terrain owners are untouched. Behaviour comes only from the public rigid-body spec and its questions. No original executable, private research, foreign worktree, or whole-operation host oracle was used.

Source milestones: `28da3b92` implements geometry, collision CFG, logical raw lists and independent literals; `c8a8c8baa5523e468d063cc00d0b6842b8b58333` bounds unresolved candidate-offset staging and records portable verification. This is bounded portable credit only: no whole-owner replacement, native rigid-body conformance, default enablement, coverage promotion or game integration. C ABI exports/version, plugin, SDK and generated ABI entries are unchanged.

## Implemented boundaries and remaining ledger

| Operation | Actual implementation/limit |
|---|---|
| §3 primary sector | Unsigned componentwise high/low minimum and exact key equality; no pointer priority or registration admission |
| §4 support point | Strict positive sign tests, zero/negative-zero ties, separate U/V store stages, unstored X sum and stored first Y sum |
| §5 contact velocity | B-at-point minus A-at-point, contact offsets and asymmetric binary32 stores; swapped order retained |
| §6.1 collision test | Four ordered separations, four support calls before candidates, transverse and velocity gates, strict improvement/tie retention and exact output-store order |
| §6.1 output contract | Early separation/no eligible candidate preserves sentinels; Q1 accepts without sentinel comparison; accepted candidates write even when final binary64 1e35 threshold returns false; Unsupported can retain prior accepted writes |
| §7.1 SweepObjectList | Forward stable logical raw storage; save next, previous.next then next.previous; free node-4 using class byte below 9 or global allocator; retain every nonzero live byte |
| §7.1 RemoveDeadObjects | Sweep roots +08,+04,+00 in order, including empty roots |
| §7.1 RemoveObject | Bounded partial operation only: unlink all raw pair matches, foreign pair-node free, conditional foreign sector-removal call, then deferred body+76 flag clear; object storage survives until sweep |
| §7.2 public advance overload | Exact three-phase composition/plan preserving dt bits. Execution stops at named missing inner operation; no successful whole timestep or fabricated completion trace |
| §6.2 ResolveCollision | Explicit Unsupported for all response, including noncoincident response; coincident random, tiny/underflow/transcendental and lifetime cases remain unsupported |
| RB1 | Constructor/allocation/copy/registration/reference/destruction unknown; AddObject returns RegistrationLifetime Unsupported |
| RB2 | Packed mapping and sector insertion/removal internals unknown; only the specifically published bounded RemoveObject foreign call is composed |
| RB3 | DoCollisions, persistent pair cycle, CheckAgainstInfiniteMassObjects and unnamed helper remain partial/unknown; CollisionCycle Unsupported |
| RB4 | Coincident random primitive/lifetime remains unknown; no host complete response supplied |
| RB5 | Inner integration, wrapping/trigonometry, drag and coordinate conversion remain partial; InnerAdvance Unsupported |
| RB6 | General x87 PC/RC, unordered and exceptional numeric domains remain unsupported; no general f64 evaluation is branded bit-exact x87 |
| RB7 (new) | Transverse h offset-store staging is not explicit. Require exact binary32 candidate-offset stores; otherwise CandidateOffsetStage Unsupported, retaining earlier writes |

Admitted numeric domain: valid independent body/output records, no concurrent mutation, finite normal binary32 fields or signed zero, nonnegative extents, caller-supplied orthonormal axes, nearest/ties-even stores and no FTZ/DAZ. Every unstored operation must be an exact dyadic with at most 53 significant bits, agreeing with both x87 PC53 and PC64 in that restricted domain. Every published binary32 store is explicit. Nonzero subnormal intermediates/stores, overflow, unresolved exact sums/products and the RB7 rounding boundary are explicit errors, never invented false/no-write results. Huge exact finite projections are supported, including the accepted-candidate/final-threshold false-with-writes fixture at extent 2^126.

Logical list admission requires complete, mutually linked, stable, distinct-root storage identities and normal nonreentrant foreign deallocation/sector calls. The host supplies raw records and specifically named foreign frees/sector primitive only; it never supplies selected normals, points, collision answers, sweep/unlink algorithms or integration results. Raw logical identities are not dereferenced as host pointers and imply no COM retention/release ownership.

## Portable fixtures and checks

All **13 new owner unit tests passed**. Public PROPOSED rows are reported here as **portable-executed only**, never native goldens:

| Published/derived rows | Recorded result |
|---|---|
| RB-P1/P2 | PASS, including swapped/mixed halves and an additional unsigned-high boundary |
| RB-S1/S2 | PASS; also negative-zero direction, d=0 tie and staged Y rounding |
| RB-V1/V2 | PASS, including swapped body order and unchanged input records |
| RB-C1–C5 | PASS, literal contacts, velocity equality/rejection, tie retention and actual store-order events |
| RB-C6/C7 | PASS, tangency/later strict improvement and bit-preserved early-false sentinels |
| §6.1 accepted/final-threshold false | PASS, derived exact-power-of-two fixture above both score constants, writes retained despite false |
| RB-L1/L2/L3 | PASS, empty and three-root order, adjacent dead nodes, exact unlink/free order, all matching pair nodes, allocator class 8/9 boundary, deferred object free |
| RB-A1 | Phase plan and unchanged dt bits PASS; actual complete three-call execution NOT claimed because inner integration is Unsupported |
| §8.2 response | NOT IMPLEMENTED/NOT EXECUTED |
| Additional numeric/mutation boundary | PASS, ties-even stores, signed zero, inexact/underflow/range rejection and RB7 Unsupported after Q1 writes |

Required baseline `qbuild all` finished exit 0: Rust **5499 passed, 0 failed/ignored**, Clippy zero diagnostics, build **217/217**, CTest **68/68**, fc4verify 1205.8s. It correctly reported tree changes during implementation and is not the stable-source gate. Focused wrapper `test` then passed **5512/0/0**, wrapper `clippy` passed with zero diagnostics and owned-file `fmt` passed. Rust-only steps avoided duplicating the still-active baseline native command; no redundant quick build was launched.

The **one canonical final `qbuild all`** ran after stable source commit `c8a8c8ba` and reached terminal **exit 0** on handle **31438**: Rust **5512 passed, 0 failed, 0 ignored**; Clippy **exit 0, diagnostics 0**; native build **PASS 4/4 edges** (59.0s), native verification **PASS** (1046.9s), full expected CTest **68/68 passed, 0 failed** (85.43s log / 85.5s wrapper). There was no final tree-change warning. No compiler/client/cache daemon was interrupted; no configuration/environment change or duplicate canonical gate was made.

Every final Rust `test result:` summary, in log order, passed counts:
`540, 3, 1, 31, 27, 40, 7, 2, 1232, 3415, 21, 13, 19, 147, 0, 2, 1, 3, 1, 1, 1, 5, 0, 0, 0, 0`.
All 26 summaries have zero failures and zero ignored tests; sum **5512**. Full logs stay in `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`; `canonical-before.json` records the source snapshot.

## Stable-source evidence and operational notes

Before and after the canonical gate, source HEAD was **c8a8c8baa5523e468d063cc00d0b6842b8b58333**, engine tree **d9581f6a44526379d50235cd96c01360ae49353a**, and all four SHA-256 values matched:

| File under engine/core/src/rigid_body_2d | SHA-256 before = after |
|---|---|
| mod.rs | `1B8DB3780D0886F1614F20D2FCAF81BAADB4C1C9E746DDC09DC26D53C08113BF` |
| numeric.rs | `E50E959D82856B29FE2D28C5206283F2C00B7038D51F375DECA4BE677659C681` |
| storage.rs | `455C2FF2296E8F63DEC7490BF42F4F9C5BCD5E521ABCD931113DBC398F912A2A` |
| tests.rs | `C86BDE8B3E94777D038068B6C6D667444C4E3604DB61FAB7276612043010939D` |

Initial base engine tree was `27a710dc5a82c241b40f9038c8fb196375e526f3`. Only this final report changes after the stable-source gate; no engine edits follow it.

Both gates stalled in optional `sccache.EXE --show-stats --stats-format json`, then completed without worker intervention. Read-only exact PID/creation/parent/cwd/argv evidence is retained in `build-runs/baseline-stall-evidence.json` (client15440, parent18484, qbuild34844) and `canonical-stall-evidence.json` (client17580 created 2026-10-23T05:28:57.142037Z, parent35428, qbuild3140). All were scoped to this worktree. A targeted helper in `build-runs/inspect-baseline-client.py` collected process parameters after CIM returned `Access denied`; no process was modified.

Preserved non-gate tooling errors: q.py formatting failed `Access is denied. Error code: Bash/Service/CreateInstance/E_ACCESSDENIED`; direct in-place rustfmt succeeded, followed by passing qbuild fmt using absolute owned-file paths. The initial relative-path fmt attempt failed `Error: file ... does not exist`, corrected by absolute paths. Git emitted `error: Unable to create 'C:/Users/tlori/FreeCity4/.git/packed-refs.lock': Permission denied` while both source commits nevertheless succeeded with the recorded HEADs; no maintenance/ref/config fix was attempted. Commits remain on the intentionally detached HEAD for supervisor publication to the reserved branch.

## Questions, game and next steps

Appended **RB7** to `docs/specs/questions/rigid-body-2d.md`; RB1–RB6 remain separately open and unchanged. Generated QUESTIONS.md was not edited; coordinator can regenerate its index. No game requests or owner native comparison were made or needed for this isolated portable-only scope; game runs and shadow/ref-warning verdicts are **n/a**, not invented passes. No active build/game request remains.

Ready for the normal merge queue as bounded core work, ABI false. Future work must answer RB7 before widening candidate-rounding admission, retain RB6 numeric restrictions until positive evidence expands them, and separately qualify response, registration, sectors, pair traversal and inner integration. Do not promote this to whole-owner/native/default support.
