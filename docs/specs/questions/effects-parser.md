# Effects text parser questions

<!-- from: core-effects-parser4 (2026-10-23) -->
- **EFP7 — Numeric qualification beyond exact finite inputs:** Which PC/RC modes,
  pre-store scalar domains, nonfinite/overflow cases and signed-zero conversions
  are positively established for scale and probability? Supply raw ParseFloat
  values and intermediate/store bits independently of parser flags/weight.
  [Portable owner currently supports explicit nearest-even, at least 53-bit
  precision; finite binary32 and bounded exact dyadic scalar stores; probability
  only exact dyadics in [0,1] with denominator at most 2^24. Other domains stop at
  a typed Unsupported boundary retaining earlier writes; no native match claimed.]
