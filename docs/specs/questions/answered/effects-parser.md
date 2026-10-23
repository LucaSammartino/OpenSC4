# Effects text parser: answered questions

- **EFP7 — Numeric qualification beyond exact finite inputs:** Which PC/RC modes,
  pre-store scalar domains, nonfinite/overflow cases and signed-zero conversions
  are positively established for scale and probability? Supply raw ParseFloat
  values and intermediate/store bits independently of parser flags/weight.
  [Portable owner currently supports explicit nearest-even, at least 53-bit
  precision; finite binary32 and bounded exact dyadic scalar stores; probability
  only exact dyadics in [0,1] with denominator at most 2^24. Other domains stop at
  a typed Unsupported boundary retaining earlier writes; no native match claimed.]
  **ANSWERED 2026-10-23 (re-effects-ef5):** DIFFERS — effects-parser.md §14
  specifies caller-supplied PC24/53/64 and all four RCs; there is no local
  control-word setup or finite/range rejection. Scale compares raw ST0 after
  storing binary32; probability has two x87 arithmetic stages and one final
  binary32 store before raw low-word extraction. NQ1–NQ17 give independent
  raw return/intermediate/store expectations, including signed zero, overflow,
  underflow, infinity and canonical quiet NaN. Results are conditional static
  predictions with masked exceptions, not measured native parser captures or
  a certification of decimal/nonfinite spellings. Reserved states/unsupported
  encodings and unmasked floating traps are explicit boundaries.
