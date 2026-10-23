# Answered questions: animated sprite

<!-- from: core-animated-sprites4 (2026-10-22) -->
- **AS8 - §3 slots 18–20 fade/highlight/self-lit:** is the accepted alpha interval closed [0,1]? When alpha
  is invalid, are both alpha and flag unchanged, or does disabling still clear flag 2? Does SetHighlight only
  set flag 1/index, or also dirty cached draw data? Are other flag bits preserved? [literal closed [0,1], invalid
  alpha leaves both fields untouched; setters preserve unrelated raw flag bits, highlight does not dirty frame]

  **ANSWERED 2026-10-23 (re-sprites-contracts4):** DIFFERS — §4.4: for completing comparisons, disabling
  always clears only flag 2 and sets alpha to 1. Enabling with −100 is a no-op; −200 disables; all other
  alpha values at most binary32 0x3F7D70A4 enable and store the input word, with no lower bound; larger
  values disable/reset. The unordered branch accepts NaN if floating comparisons complete. Returns 2.
  Highlight sets/clears only flag 1, narrows its full-integer index to a byte, returns 0 only for an exact
  state match and otherwise 2. Self-lit uses the supplied byte and flag 4, returning 0 for an exact normalized
  state match and otherwise setting/clearing that bit and returning 2. All preserve unrelated flag bits;
  none dirties frame/view state. Evidence rechecked with asm --bytes at 007FB340–007FB3A8,
  007FB3B0–007FB401, 007FB410–007FB44B. Boundary: this answers setter behavior on the completing
  normal path plus the stated unordered branch, not unmasked floating traps/nonfinite whole-call modes
  (AS7), lifetime/inventory (AS3), native execution or default/runtime qualification.
