# Answered questions: animated sprite

- **AS4 - §2 loading:** resource type/IID, zoom order, ownership and failed/repeated load effects.

  **ANSWERED 2026-10-23 (re-animsprite-as5):** DIFFERS — §2.1 distinguishes type `0x09ADCD75` from
  IID `0x69B6F01C`; all five ordered acquisitions plus a non-null texture binding are required for true.
  A failure releases/nulls this call's successful prefix in reverse; it does not retain that prefix or
  recover overwritten old interfaces. Untouched suffix, old binding before the binding stage, copied
  count/scales and derived metrics survive as specified. No playback/controller reset. The published
  transaction was rechecked against instructions at `007FD120–007FD2E9` and `007FC320–007FC43F`.
  Numeric/malformed-record qualification is separately tracked by AS6/AS7.

- **AS5 - §§3–4 clock and sequence stepping:** sentinels, sequence boundaries, reverse/empty/single-frame,
  return values and skip-loop progress.

  **ANSWERED 2026-10-23 (re-animsprite-as5):** DIFFERS — §§4.2–4.3 publish byte/u16 stepping and
  conditional signed remainder, literal rate storage (Tick replaces −100 by −200), no resource rate,
  no SetPlayMode reset, post-advance direction subtraction, AdvanceFrame return 2 and Tick return 0/2.
  There is no convergence guarantee or native error result for the skip loop. Added signed high-count,
  reverse ping-pong, stop, single/empty and non-progress vectors. Verified `007FB1E0–007FB306`,
  `007FB310–007FB33C`, `007FCEA0–007FD0B7`, controller helper `007FAE90–007FAF34`.
  General arithmetic qualification and controller installation remain AS6 and AS1 respectively.

- **AS11 - §2 foreign factory route:** IID, options, output and reference transfer for type `0x5AD0E817`.

  **ANSWERED 2026-10-23 (re-animsprite-as5):** DIFFERS — §2 requests IID `0x9AD0E7E3` with zero/zero
  options and a null-initialized temporary. On success resource slot 4 receives `(0, caller-output)`;
  the resource is released exactly once and its boolean/result passed through without additional
  model QI/AddRef. Acquisition failure leaves caller output untouched. Verified `007FD7D0–007FD81E`.

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
