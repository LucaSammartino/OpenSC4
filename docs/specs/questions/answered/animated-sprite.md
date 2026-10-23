# Answered questions: animated sprite

<!-- from: core-animated-sprites4 (2026-10-22) -->
- **AS4 - §2 loading:** AS1 remains open; model-instances §6 identifies a 0x09ADCD75 table queried via
  IID 0x69B6F01C, whereas §2 calls that IID a resource type. Confirm the exact Load/LoadFromResource inputs,
  key type/group rewriting, zoom iteration order, reference ownership, missing-first/missing-later zoom return
  values, rollback versus retained prefix, repeated-load replacement, and which fields initialize/reset.
  [typed foreign decoded records; acquire in array order with IID 0x69B6F01C, stop at first absent zoom;
  expose prefix/stopping reason, do not call a prefix a successful native Load or implement resource readers]

  **ANSWERED 2026-10-23 (re-animsprite-as5):** DIFFERS — §2.1 distinguishes type `0x09ADCD75` from
  IID `0x69B6F01C`; all five ordered acquisitions plus a non-null texture binding are required for true.
  A failure releases/nulls this call's successful prefix in reverse; it does not retain that prefix or
  recover overwritten old interfaces. Untouched suffix, old binding before the binding stage, copied
  count/scales and derived metrics survive as specified. No playback/controller reset. The published
  transaction was rechecked against instructions at `007FD120–007FD2E9` and `007FC320–007FC43F`.
  Numeric/malformed-record qualification is separately tracked by AS6/AS7.

<!-- from: core-animated-sprites4 (2026-10-22) -->
- **AS5 - §3 slots 9–16, §4 clock and sequence stepping:** please specify animation-table layout/step rules,
  SetAnimation invalid-index and state-reset behavior, SetRate(-100/-200) resolution and raw resource rate
  field (no inferred time conversions), SetPlayMode side effects, AdvanceFrame at reverse frame 0 in every
  mode, and single/empty-frame cases. Does clock subtraction use direction before or after reversal? What
  are native Tick/AdvanceFrame returns when the frame does not change or playback stops? How is the literal
  skip loop made to terminate for a single frame, negative rate/elapsed or zero direction with |position|>=1?
  [only explicitly named unsequenced operations; raw elapsed*rate*direction, literal advance-then-subtract,
  no-skip long-tick position=1; unresolved sentinels/reverse boundary/non-progress return errors transactionally;
  no native success return invented and no sequence-table stepping delegated to a foreign owner]

  **ANSWERED 2026-10-23 (re-animsprite-as5):** DIFFERS — §§4.2–4.3 publish byte/u16 stepping and
  conditional signed remainder, literal rate storage (Tick replaces −100 by −200), no resource rate,
  no SetPlayMode reset, post-advance direction subtraction, AdvanceFrame return 2 and Tick return 0/2.
  There is no convergence guarantee or native error result for the skip loop. Added signed high-count,
  reverse ping-pong, stop, single/empty and non-progress vectors. Verified `007FB1E0–007FB306`,
  `007FB310–007FB33C`, `007FCEA0–007FD0B7`, controller helper `007FAE90–007FAF34`.
  General arithmetic qualification and controller installation remain AS6 and AS1 respectively.

<!-- from: core-animsprite5 (2026-10-23) -->
- **AS11 - §2 foreign factory route:** for GetSpriteAnimFromResourceKey (type 0x5AD0E817), what IID,
  resource-manager options, output interface and reference-transfer rules accompany the published
  resource-manager fetch? [routing is preserved; no guessed successful foreign factory. The sprite
  construction/load route for type 0x29A5D1EC is implemented separately.]

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
