| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-parser4: bounded portable parser | Codex / core-effects-parser4 | ready | Land bounded owner; EFP1–7 remain open |

## Changes and limits

Added `engine/core/src/effects_parser/` and one additive core lib module declaration.
The earlier core-effects4 report explicitly excludes this owner; no contrary
same-owner module/history appeared. Start/base was `5fa5dbe8147531e9a8dafca083602370f820c7fe`.
Production milestone: `50afe85c`; stable qualification source: `e2b311ce`.

| Boundary | Implemented portable work | Limits |
|---|---|---|
| ParseDescRecOptions | Exact finite lookup order, offset word copies then flags/count, original-value scale comparison after store, LOD priority/default/wrapping, count-dependent emit/size conversions, probability arithmetic/store/low bits, length precedence | Partial owner: five rotation branches stop at their actual lookup; error/unsupported paths retain prior writes and calls, never roll back or manufacture AL success |
| Named/anonymous append | Manager-null short circuit; shared scratch name reread after raw validation; select low word/type/initial bit; options before local append; duplicate names append; raw anonymous registration index then full type word | Foreign constructor/string/copy/insertion primitives; no foreign identity, counter, collision, ref-transfer or unwind policy invented |
| Manager/QI | Ordered AddRef/store/Release, same-manager QI, temporary reference sequence, null manager retains K, null QI clears K, unconditional reset; null-out QI and owner/fallback branches | Typed logical identities, including distinct adjusted interface/canonical unknown identities; physical pointer/table binding unsupported; no Drop-based logical reference releases |
| Commands | Base registration then 21 ordered literal/descriptor associations with direct/block allocation branches; separate sequenceEffect association and concrete nonempty-block/valid-selection type-6 append | Registrar/dictionary lifetime foreign; other handlers, block grammar and dispatch policy unsupported |

Reuses `effects::ComponentRecord` without changing its owner/defaults. `type_word`
retains the complete +0C scalar; `component.kind` is an explicit low-byte projection.
The shared constructor's semantic field facts are supporting foreign inputs, not
parser-exclusive coverage. Placement padding zeros and Vec capacity/copies are
portable memory choices, not native padding, pointer-stride or allocation claims.
Typed UTF-8 token strings represent stipulated payload identities, not lexer acceptance.

Numeric support explicitly requires nearest-even binary32 stores and at least
53-bit arithmetic precision. Finite binary32 replies (including signed zero) and
bounded exact dyadic pre-store replies are supported for scalar stores. Dyadics
have i32 coefficient, exponent -149..104 and finite binary32 store. Probability
admits only the reviewed raw dyadic representation with exponent >= -24 and value
in [0,1] (binary32 replies use their explicit significand/exponent decomposition,
with zero handled exactly). Multiplication/addition are exact in that domain;
the owner performs the binary32 store and low-16 extraction. Broader PC/RC,
nonfinite, overflow and unqualified numeric inputs stop as Unsupported.

No complete lexer, factory, whole constructor/destructor, runtime effects,
manager/emitter/collection/bootstrap/queue algorithm, native vtable binding,
C ABI export, default map or coverage credit was added.

## Measured validation

- Before production comparisons, independently transcribed all P1–8 and
  R1–6/M1–4/Q1 into 19 fixture groups. Added 12 scoped boundary/registration groups.
  Focused portable execution: **31 passed, 0 failed**, terminal RC 0. These are
  stipulated primitive-input tests; the published native vectors remain **UNEXECUTED**.
- One canonical `qbuild all`, no baseline/quick or repeated full gate:
  **5,530 passed / 0 failed / 0 ignored**, summed over **26** Rust result lines;
  Clippy **RC 0, zero diagnostics**; native build **217/217 edges**;
  full CTest **68/68**, 153.73 s; fc4verify **PASS**, 412.2 s;
  actual qbuild terminal **RC 0**. Workspace count includes the parser tests;
  the separate focused run is not added to that aggregate.
- Gate start/end HEAD identical: `e2b311ce72768c192f4ea22525b0b89f08ada2b1`.
  Engine tree identical: `dc23eaa25d67daf9110bc84bc4aba858058a279a`.
  All four touched Rust file SHA-256 values identical before/after, engine status
  clean at both boundaries. Evidence: `build-runs/parser-gate-evidence.json`;
  raw logs: `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-*.log`.
- Kept the same async gate handle through terminal. Optional stats client
  PID 10084, creation 1792734901.196986, parent fc4verify 21984, was observed
  waiting, then absent (cause unclaimed); same qbuild 31048 continued to terminal.
  Exact cwd/argv/parent/creation and owning operation are preserved in
  `build-runs/parser-gate-process-evidence.json`; client port was not observed
  before it disappeared. No cancellation, restart, cache RPC or process mutation.
- q.py focused launch initially failed before cargo because Bash access was denied;
  used the local quiet subprocess runner with own role/path/manifest/R1 checks.
  qbuild fmt passes the three parser files. Checking lib.rs recursively exposes
  existing lot_developer formatting differences; none were changed. An initial
  fmt invocation used worktree-relative paths where the wrapper expects engine-relative paths.
- No game request or native parser oracle was run; game verdict/shadow: n/a.
  No C ABI changes.

## Questions and next steps

Appended **EFP7** to `docs/specs/questions/effects-parser.md` for broader numeric
qualification. **EFP1–6 remain open and unchanged**. The coordinator may regenerate
the questions index; this lane did not edit the generated index or run tools/.
Land this bounded portable owner; obtain independent public contracts/oracles
before extending rotation, numeric controls, error transport, foreign identity,
command grammar or final lifetime. No further gate or game run is requested here.
