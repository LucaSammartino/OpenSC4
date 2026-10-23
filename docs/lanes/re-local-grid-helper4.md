| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-local-grid-helper4 (native local grid helper spec) | `Codex` / `re-local-grid-helper4` (RE docs lane) | DOCSREADY, bounded: helper spec, G1–G7/B/L1–L14 proposed traces; questions/doccheck/link/diff/analytical checks PASS; LGH1 open; ABI unchanged; no game runs | (1) preserve LGH1's ambient FP/foreign declaration limit; (2) no native coverage promotion from six provisional entries/538 bytes. |

## Report

Published [local-grid-helper.md](../specs/local-grid-helper.md), the first dedicated
helper specification. It owns the sequencing and state mutations; resource
creation, effect operations, raw scalar replies and scalar floor remain explicit
foreign primitives. Public zone-tool/city-tools-input documents link to it;
their caller algorithms were not expanded.

### Findings and provenance

- SetPosition uses five separate scalar replies, two floor calls and one
  transform dispatch. It snaps x/z with explicit binary64 division/product
  stores, adds fresh replies divided by 16, copies y's 32-bit word, and stores
  scale as 16 times the fifth reply. Equal input/cell and hidden state do not
  suppress updates. No dimensions or orientation argument exists.
- The 56-byte descriptor has an identity 3×3, translation and scale, byte 0=2,
  and flag byte 1 or 3 chosen from the held x87 scale comparison. Padding +2/+3
  is uninitialized; whole-buffer byte equality is not claimed.
- Init requests `local_grid` directly into H only when H is null; factory
  status is ignored. ShowEffect queries activity and optionally sends Start(0)
  or Stop(0). Shutdown queries, optionally stops, clears H, then releases.
  Destructor-labeled cleanup releases without Stop or field clearing.
- Six non-thunk entries/538 bytes remain provisional metadata. Direct bodies
  and four named consumer families corroborate helper use; tiny constructor
  and destructor exclusivity/folding and exact C++ declarations are qualified.
  No production-module, owner/byte/default/native/runtime coverage promotion.
- Canonical export queried first, then instruction bytes and PE constants.
  PE SHA256 is `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`;
  export file hashes are in spec §1. Decompilation was navigation only.
  No REA call or fallback was needed.

### Independent evidence and checks

All traces are PROPOSED/UNEXECUTED: G1–G7 include asymmetric signed noninteger
positions, independent replies, unity/nonunity scale and stored-unity/held-
nonunity flags; B covers signed zero, negative/positive coordinates and exact
cell boundaries; L1–L14 cover retained/null factory output, ignored status,
repeat Init/show/hide/moves, identity changes after teardown, Shutdown and
destruction. No native, portable, device, effect, game or renderer was executed.

- `python build-runs/local-grid-analytic.py`: PASS, exact rational checks for
  G1–G7 and B, 28 binary32 words, 15 relative file links including this report. This checks fixed
  literals, not a host geometry oracle or future implementation output.
- `python tools/questions.py index`, then `check`: PASS; LGH1 indexed.
- `python tools/doccheck.py check`: PASS.
- `git diff --check`: PASS.
- Cargo/clippy/ctest/game/shadow: n/a (docs-only); no ABI rows or exports.

Retained operational errors: q.py's Bash launch returned E_ACCESSDENIED;
initial qgrep output raised a cp1252 encoding error (subsequent calls used
UTF-8); a multi-address callers command was rejected and rerun per address.
Detached commits report shared `packed-refs.lock` Permission denied while
successfully creating their detached HEAD commits. Shared refs/config were
not changed to suppress that message.

### Questions, milestones and disposition

[LGH1](../specs/questions/local-grid-helper.md) remains open for the foreign
scalar's declared precision and guaranteed ambient FP environment. The spec
publishes conditional exact vectors and explicit stores without claiming
arbitrary-bit native equivalence. Null globals, malformed storage/interfaces,
foreign exceptions, callback mutation and repeated destruction are unsupported,
not invented expected-false cases. The helper's normal-domain branches are
specified; this is bounded documentation readiness, not full native coverage.

Milestones: `fd690b95` lane scope; `ac0468a1` transform and qualification ledger;
`046cf00a` lifecycle and traces; `6d294412` analytical/docs checks and question
index. Authority was the own-lane RE round-1 manifest at
`C:/Users/tlori/FreeCity4/build-runs/run4-re-local-grid-helper4-r1-manifest.json`;
runtime guard remained within the supplied budget/caps. No subagents, shared
queue/state edits, merge/rebase/push, source/tool/SDK/ABI changes or native runs.

Next: retain LGH1 until positively proved in a separately authorized scope;
implementers can use the bounded helper rules with explicit primitive inputs.
