| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lua-lh6: Lua interception and replay contracts LH6-LH7 | Codex; re-lua-lh6 (detached worktree) | ready: LH6/LH7 answered; doccheck PASS | Land specs; keep full replay disabled and bounded lua-registry credit separate. |

## Report

Answered both owner questions with original behavior and an explicit verification scope:

- LH6: §§1.4.2 and 13 establish that the specified public interfaces have no complete
  record-only observer. Factory creation, retained views, debug hooks, metamethods and
  lifecycle/control callbacks do not cover every owner before Open or every subsequent
  interface/table operation. This is a negative answer, not an invented installation recipe.
- LH7: §§3.5 and 9.1 describe external library/foreign inputs, CRT randomness shared on the
  same OS thread, observable identities, registry delivery and iteration order. The original
  supplies no replay exclusion/injection/normalization facility. Full replay remains disabled.
- §8.6 defines the bounded collector/registration comparison: 47 ordered records (3 global,
  44 game), flags, exact non-null callback entry points, scratch callback and stack balance.
  No nondeterminism exclusions; executable rebasing is a portable address label only.
  Closure internals, scripts and unsampled VMs receive no verification credit.
- §16.2 adds 13 prose conformance vectors (LH6-A–F, LH7-A–G), specified, not executed.
  LH6/LH7 were moved intact to the answered archive with MATCHES annotations; questions
  index regenerated. No open lua-host entries remain; legacy §15 questions are unchanged.

### Evidence and limits

Used main-checkout tools/re_query.py before decomp searches; all new low-level assertions
were checked against export instructions with bytes. Key sites: factory `008800A0`, automata
factory/Open `0051C4E4–0051C4FC`, ScriptServer factory/Open `008B08A1–008B08BD`, public
GetTable `008803C0/008B7B68`, VM direct lookup `008C6C28`, hook setter `008B9770`, Lua
random/seed `008C09FC/008C0B32`, CRT OS-thread state `009EFB92–009EFBC0`, clock/environment
`008C3471/008C3450`, and array/hash traversal `008C8F90–008C906C`. Existing collector layout
and registration rules are cited from advisors §8.2 and lua-host §§8.1–8.2. No REA request was
needed; no missing-target error or corrected-source result is represented as evidence.

The supplied glue-lua-shadow5 report in this worktree, main and the owner worktree contains
no Owner closure table or `open:` rows. Its Questions section and the filed LH6/LH7 entries
were the available owner blockers reviewed. The answer closes the native-behavior questions;
it does not make full replay feasible under the stated access constraints. A future full-replay
capture/isolation design is separate work.

### Validation

- `python tools/questions.py index`: regenerated index, open lua-host count 2 → 0.
- `python tools/questions.py check`: PASS.
- `python tools/doccheck.py check`: PASS.
- `git diff --check`: PASS.
- Cargo, clippy, CTest and game runs: n/a, spec-only lane. No engine, plugin, capi,
  coverage-map or ABI edits; no ABI-row proposal and no new measured shadow result.
- Spec milestone: `bece9c71c`. Git printed a packed-refs.lock permission diagnostic, but
  the detached commit succeeded and subsequent HEAD/status inspection confirmed it.

## Handoff

- Goal: answer LH6 and LH7 in lua-host, with conformance vectors; no engine changes.
- Done: spec/archive/index milestone `bece9c71c`; evidence review and final report complete.
- Gate: questions check, doccheck and whitespace check PASS; no executable tests claimed.
- Next: coordinator lands queued detached HEAD; implementers use §§8.6/16.2 for bounded scope.
- Full replay remains disabled; native observer absence is answered, not a remaining RE question.
- No game requests, ABI proposals or pending source queries.
