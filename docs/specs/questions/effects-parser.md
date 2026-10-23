# Effects text parser questions

**EFP2 — Rotation instruction/store semantics**

In `effects-parser.md` §2/§8, establish the x87 operation and binary32 temporary/store
order for all five rotation branches of `005989D0`; distinguish their key order
from text order. Current supported domain requires those lookups absent.

**EFP3 — Error transport and lifetime outside normal domain**

In `effects-parser.md` §3, identify the actual error transport of `009EF84F` and
cleanup/unwind behavior of the select/LOD/prob errors. Do not assume false-return,
transaction rollback or resumed parsing from the misleading export name.

**EFP6 — Command-library dispatch policy**

In `effects-parser.md` §6, establish foreign root/nested registrar duplicate,
lookup/case, descriptor ownership and block-hook dispatch policy. Current proof
is the owner's ordered literal-to-descriptor registration, plus the concrete
`sequenceEffect` normal handler, not all root command semantics.

<!-- from: core-effects-parser4 (2026-10-23) -->
- **EFP7 — Numeric qualification beyond exact finite inputs:** Which PC/RC modes,
  pre-store scalar domains, nonfinite/overflow cases and signed-zero conversions
  are positively established for scale and probability? Supply raw ParseFloat
  values and intermediate/store bits independently of parser flags/weight.
  [Portable owner currently supports explicit nearest-even, at least 53-bit
  precision; finite binary32 and bounded exact dyadic scalar stores; probability
  only exact dyadics in [0,1] with denominator at most 2^24. Other domains stop at
  a typed Unsupported boundary retaining earlier writes; no native match claimed.]
