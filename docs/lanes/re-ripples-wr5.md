| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ripples-wr5: ripple owner contracts | Codex; re-ripples-wr5, detached checkout | ready: spec changes; doccheck passed | Apply WR1/11–14 answers and direct-call ledger; WR8 remains partial, WR2/3 open |

## Report

Updated [water-ripples.md](../specs/water-ripples.md) with binary-supported behavior and six proposed
conformance cases (26–31). No implementation files changed. The requested module Owner closure table and
core-ripples5 report were read; no engine source was read or edited.

### Answers and measured facts

- **WR1, answered:** integer tick elapsed, period and rebuild threshold are microseconds. The animation
  manager constructs unit-3 timers and passes the bounded integer delta; ripple does not use its scaled float.
- **WR11, answered:** stored factor at `00AA8C60` is binary32 `3C8EFA35`, exactly
  `0.01745329238474369049072265625`. A 60-degree float transforms to `3F860A92`.
- **WR12, answered:** executable initializer `00A751D0`, table entry `00B07680`, constructs the shared RNG
  from host cRZTime XOR `8C09C57A`. Seed `FFFFFFFF` uses the native host-time/clock fallback. No ripple
  constructor, Init, Shutdown or tuning reset reseeds it. A captured/injected seed is a fixture input,
  not a requirement for the original owner to reject an uninitialized process RNG.
- **WR13, answered for the documented valid arithmetic domain:** §1.3 specifies ordered spline weights,
  asymmetric coordinate/derivative sums, every binary32 spill versus retained x87 intermediate, normalization,
  sine/phase/vertex ordering and turn dot/acos evaluation. Exactness requires matching caller control word
  and x87 transcendental behavior; no host Rust math or tolerance fixture becomes native bit equality.
- **WR14, answered:** delayed tick clears pending at `00761E9F` before clearing/detecting/recreating,
  giving one detection. Pending immediate message still detects twice. The delayed rebuild clock is retained.
- **WR8, partial:** §2.5 adds the ripple direct-call input/result/receiver checklist, retains published
  defaults/lifecycle/tuning/resource contracts and separates object methods from storage helpers.
  In particular, `007606B0` takes a strip-list receiver, not a ripple object. A fresh export measurement
  reproduces selection-window 25 bodies/5838 bytes plus ripple-window 44/9438 = **69/15276**.
  The historical **65/15308** lacks a supplied membership manifest and cannot establish owner closure.

WR1 and WR11–WR14 moved to the answered archive with dated lane annotations; `tools/questions.py index`
regenerated the index. WR8 deliberately remains open for unreconciled historical attribution, remaining
storage/insertion signatures and foreign text/object/exception, allocation, alias and reentrancy domains.
Pick's four semantic types remain opaque; its false/no-output behavior is established. **No complete-owner,
native runtime, byte coverage or engine closure is claimed.** WR2/WR3 remain open: this pass adds no proved
semantic names for flags/selection bytes beyond their existing literal behavior. No new question IDs filed.

### Validation and evidence

- `python tools/doccheck.py check`: **PASS**. `git diff --check`: **PASS**.
- Cargo, clippy and CTest: n/a, spec-only changes. No game runs, requests or shadow comparison.
- Vectors 26–31 are **unexecuted proposals**, not native captures or implementation validation.
- Primary evidence: main-checkout read-only export queries, targeted decompilation reads and PE constant
  bytes. Numeric rules were checked against `re_query.py asm --bytes`. No REA call was needed.
- Local ignored research helper: `build-runs/evidence.py` (PE constants/export window counts). No RE material
  or helper code was committed. No ABI exports changed; no proposed ABI rows.
- Commits: `20104fc0` initial checkpoint; `94060005` clocks/RNG/factor/pending;
  `349f277d` arithmetic ledger/direct-call checklist. Git emitted a denied shared packed-refs lock warning,
  but each detached-HEAD commit succeeded; no shared ref/config change or workaround was attempted.

## Handoff

- Spec changes are ready to land; the original request's full WR8 closure is not achieved.
- Engine follow-up: use constant 3C8EFA35, process startup RNG source, one delayed detection, and §1.3
  arithmetic ledger; retain the distinction between ordinary arithmetic support and bit-exact x87 support.
- Coordinator: obtain the historical 65/15308 membership manifest before reconciling attribution; commission
  focused WR8 foreign/storage residual work if full native owner closure is still required.
- WR2/WR3 require separate semantic evidence; no flag/byte names were guessed here.
- No game request, ABI row proposal or implementation gate outstanding for this docs delivery.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-ripples-wr5-r1-manifest.json.
