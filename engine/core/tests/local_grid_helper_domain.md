# Qualified portable arithmetic ledger

Source: public local-grid-helper §§2–6, LGH1 (still OPEN). This is a
deliberately qualified portable API, not a claim about ambient native x87,
service return declarations, exceptions, rendering, callers, or whole-byte/native coverage.

## Accepted inputs and primitive boundary

- Select `QualifiedDyadicNearest` explicitly. Unqualified mode returns typed
  Unsupported, without reading inputs; the null-H guard precedes qualification.
- x/z: finite binary32, exact multiples of 2^-30, absolute magnitude <=2^16.
  y: any finite binary32 word, copied without arithmetic (including negative zero).
- s1/s3: positive powers of two in [2^-16,2^16].
- s2/s4/s5: exact finite binary64 dyadics that are multiples of 2^-30,
  absolute magnitude <=2^16. Signed zeros and negative nondivisor replies are supported.
  Replies describe raw ST(0) inputs; G7 is not squeezed into a presumed f32 return.
- Floor replies must be the finite ordinary integral floor of the actual
  binary64 argument, with the ordinary signed-zero result for zero inputs.
  The host provides each raw floor reply; validation never substitutes a reply.
- Nonreentrant synchronous primitives with stable helper/inputs/interfaces,
  except for Init passing its actual H field as the raw writable output channel.
- Nonfinite/out-of-domain primitive replies or positions return Unsupported at
  the first reached qualification failure. The completed prefix includes capture,
  position reads, individually numbered scalar and floor calls. No transform
  follows a failure; this is not a native expected-false or fault contract.

## Bound proof, checked from actual operations

1. A raw scalar on lattice 2^-30 of magnitude <=2^16 has integer coefficient
   <=2^46, hence at most 47 significant bits. It is exact in binary64 and the
   required s1/s3 R64 stores lose no bits. The lattice check multiplies by 2^30
   exactly and tests the fractional part. Tiny off-lattice values cannot pass:
   the exponent shift remains representable even for the smallest f64 subnormal.
2. Division of a binary32 x/z by a power of two shifts its exponent, retaining
   at most 24 significant bits. The quotient magnitude is <=2^32; nonzero
   magnitude is >=2^-46. Thus each actual Q R64 store is exact, away from both
   exponent extremes. Negative zero survives division and the floor argument.
3. Ordinary floor gives a finite integer of magnitude <=2^32, at most 33
   significant bits. Multiplication by the same power-of-two divisor is exact;
   the P R64 store is exact. A conservative bound is |P|<2^17; the product is
   on lattice 2^-16. No integer cell conversion replaces these operations.
4. Each fresh s2/s4 offset multiplication by 2^-4 is exact (<=47 bits), on
   lattice 2^-34 and of magnitude <=2^12. P plus that offset is on lattice
   2^-34; the absolute integer coefficient is <2^51+2^46<2^52. Consequently
   the separate add needs at most 52 significant bits. This independently
   verifies the critical bound, rather than assuming every dyadic add is exact.
   A nonzero result is >=2^-34, and |result|<2^18: no underflow/overflow.
5. s5 times 16 is exact, on lattice 2^-26, magnitude <=2^20, <=47 significant
   bits. Compare this held product with 1 independently of the R32 store.
6. Binary32 stores extract binary64 sign/exponent/significand, round the low
   29 bits by integer nearest-even logic, handle significand carry, and pack a
   normal binary32 word. Every admitted nonzero result is normal binary32.
   No host cast or ambient rounding setting supplies the R32 rounding oracle.
7. Separate multiply/add expressions have exact nonzero magnitudes, so no
   ambient precision is claimed or needed. The nearest-mode sign of an exact
   zero sum is chosen explicitly: cancellation is +0, (-0)+(-0) is -0.
   This does not claim general native PC64 or exception-state equivalence.
   No global x87/MXCSR/thread environment is read, written, or restored.

## Applicability of every public vector

G1/G2/G3/G5/G6 have integral power-of-two divisors in [4,16], offsets <=64,
and finite binary32 positions of magnitude <=17.5. G4 divisors/replies are
2^-4 and positions 3*2^-5, -3.25, -2^-5. Every value is on the admitted lattice.
G7 s5=2^-4+2^-30 is exactly representable in binary64 with 27 significant
bits, fits the raw reply domain, and produces held 1+2^-26 (not unity), then
stored 1 under nearest-even. No published G row is omitted.

All seven B x values lie in [-16,16] and are multiples of 2^-30. The smallest
noninteger boundary increment is 2^-20. Signed-zero x/y and floor replies are
explicitly retained. Each Q/F literal is within the floor domain above.

The seven G descriptors contribute 28 independently transcribed output words;
the seven B cases have separate literal floor inputs/replies and Tx words.
L1–L14 use only raw creation/query replies and primitive event ledgers. The
L10 changed-x floor input is independently 71/64 for x=17.75; it shares G1
output, not its quotient. Additional tests exercise extrema, off-lattice and
nonfinite rejection, every scalar prefix, invalid floor replies, and R32 ties.

## Semantic descriptor and lifetime

Typed fields describe only 54 defined bytes: header at 0/1, identity at 4..39,
translations at 40/44/48, scale at 52. `DEFINED_BYTES` excludes +2/+3.
Projection preserves caller backing bytes at +2/+3 as a portable choice;
it neither reads uninitialized padding nor promises native padding contents.
The transform view is synchronous and temporary. Tests compare every defined
byte, including the nine identity words, with two distinct padding sentinels.

Shutdown Release receives a read-only observation of H after the clear;
destroy consumes the helper and Release observes the retained H. This observer
is a portable lifecycle test seam, not an extra native Release argument.
There is no implicit Rust Drop foreign call, helper refcount, or reentrancy claim.

## Open boundary

LGH1 remains the published question. No new behavior ambiguity was required
to implement this requested subset. Broader native/ambient/exception domains
remain Unsupported. All proposed native vectors remain UNEXECUTED. No ABI,
plugin, default, map, caller, region, or renderer behavior is implemented here.
