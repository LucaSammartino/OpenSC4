| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span8-impl4 (span8 own-type bridge) | `Codex` / `glue-span8-impl4` (engine lane, detached transport) | Blocked on SR9/SR10; findings dd76097e, baseline 0eca0e80; final cargo4173/0, clippy0 diagnostics, ctest67/67 PASS; no ABI/runtime changes, no game runs or coverage credit | (1) Reconcile native ABI and land span8 CAPI; (2) implement opt-in bridge with row13 refusal; (3) native fixture and 2032/ref0 checks, then literal-list union. |

## Result

The requested replacement cannot be connected through the available ABI. The task explicitly requires stopping
paths with missing ABI and prohibits unreconciled ABI edits. This round therefore commits findings only;
`enable = span8` is not installed or advertised, and existing sibling renderer behavior is unchanged.

Read the scoped kernel/descriptor and section 10 bridge contracts, SR8-2/3/4, the shared FC4SpanContext
infrastructure, existing 4444 bridge, public ABI tables and context proposal. No forbidden sources were read.
Findings landed in dd76097e before call30; baseline handoff landed in 0eca0e80.

## ABI prerequisites and questions

- **SR9**, appended to `docs/specs/questions/span-render-8.md`: renderer vtable `0x00AC188C` has no reconciled
  public rows, nor do the native buffer/context/draw-system classes needed by the fixture. Its six slots require
  thiscall pops 4/16/16/20/16/24, destructor EAX and void draw returns. The existing 17-row
  `docs/abi/proposals/re-span-context4.tsv` is explicitly unapplied; context slot22 still has return `unknown`
  while section10.5 specifies void. The question lists the precise route slots/pops needed.
- **SR10**, same file: the landed Rust core has solid/gradient/alpha/alpha-gradient/blit operations and
  `render_blt_aliased`, but no span8 adapter/export exists in CAPI modules, `lib.rs` or `fc4.h`. A public adapter
  must preserve bounded signed-stride/origin descriptors, transactional rejection and ascending alias order.
- IDs follow the generated index's next free `SR9`. The index was not edited or regenerated because this lane
  may only append its question file and may not run `tools/`; coordinator regeneration remains required.

## Validation

- Baseline `qbuild all`: cargo4173 passed/0 failed, clippy0 diagnostics, native build205/205, ctest67/67.
  Baseline reported tree changed during run because the findings documentation was committed during it.
- Final `qbuild all` on clean commit0eca0e80: **PASS**, cargo4173/0, clippy0 diagnostics, native build2/2
  cached edges (0.8s), ctest67/67 (86.0s), fc4verify87.1s. No tree-change warning.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- No Rust or plugin files changed; no formatter check or runtime fixture was applicable. No C ABI exports changed.
- No game requests submitted, none outstanding. No native-context counters, shadow exactness, ref0 result,
  2032 run or union regression is claimed. This is WIP, not ready for runtime promotion.

## Continuation

Reconcile SR9 and land the approved SR10 adapter first. Then implement the opt-in exact own-type bridge with
callee pops unchanged and slot0/game lifetime untouched; retain SR8-3 row13 refusal. Extend FC4SpanContext
counters additively, separating constructed native-context calls from shipped calls. The only known heightfield
producer bypasses this renderer (section10.6); constructed fixtures provide no whole-class reachability evidence.

Run a uniquely labelled native fixture through2032/ref0, and only after clean supported results request the
coordinator-supplied current-list+span8 union. No default promotion or coverage credit. Remain within the
coordinator's three-round cap; this result uses one round. The detached commits are for supervisor publication;
no branch switching, ref edits, merge or push was performed.
