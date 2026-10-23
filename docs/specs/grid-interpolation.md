# Byte-grid interpolation

## 1. Scope and evidence

This is the owned numerical contract of the two Windows 1.1.641.0
`SC4InterpolateGrid` bodies. It specifies byte grids, not pixels, colours, terrain
resampling, or a host-supplied interpolation result. It supplements
[map-view §4.3](map-view.md#43-interpolation-lots-and-the-3d-overlay).

The original `bin/SimCity4.exe` SHA-256 is
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`,
matching `ghidra/export/windows/export-summary.txt`; image base is `00400000`.
All numerical claims below were checked against byte-bearing instructions from
that export/original pair. Local decompilation supports the two-argument shape
only; it is not the numerical authority. No native execution was performed.

| Entry | Attributed range / size | Input interpretation | Output |
|---|---|---|---|
| `0079EB40` | `0079EB40–0079ED8A`, 587 bytes | unsigned byte, 0…255 | interpolated unsigned byte |
| `0079EFD0` | `0079EFD0–0079F220`, 593 bytes | signed byte, −128…127 | interpolated signed value plus 128, stored as a byte |

The range endpoints are inclusive. Export search identifies exactly these two
named instantiations. Both are contiguous bodies with no direct callees, tail
delegation, or data xrefs to their entry points. Their sole direct callsites are
`007A2489` and `007A2840`, respectively. The bodies are distinct, not folded
aliases: the signed loads and final bias instructions differ. Other functions
in the surrounding address interval are not attributed to this contract.
The exported template names are attribution aids, not proof of an algorithm.

## 2. Arguments, storage and ownership

Both take two stack arguments, in this order: destination grid descriptor,
source grid descriptor. They return with plain `RET`; the caller discards eight
argument bytes (`007A247F–007A2490`, `007A2836–007A284B`). There is no defined
result/status value in a register. These are internal functions, not proposed
public ABI signatures.

A grid descriptor has a 32-bit begin pointer at offset 0 and end pointer at
offset 4. Its elements occupy 12 bytes each. Each element has its own byte-array
begin at offset 0 and end at offset 4. The functions do not read capacity fields.
They obtain the outer count by signed division of the outer byte-pointer
difference by 12 and the inner count by subtracting the first element's byte
pointers (`0079EB43–0079EB89`, `0079EFD3–0079F019`).

Use **outer** and **inner** as axis names here; no image row/column orientation
is imposed. A literal matrix below lists outer elements as bracketed rows and
inner elements left to right. `S[a,b]` is a byte in source outer element `a`,
inner index `b`. Let source counts be `A,B`, destination counts `C,D`.

The outer descriptor stride is 12 bytes. Within one element, the byte stride
is one. There is no shared row pitch: each element has an independent begin
pointer. Only the first element's length determines each grid's inner count;
later lengths are not checked. A rectangular grid with every referenced element
backed by at least that many bytes is therefore a precondition, not a validation
result. Destination dimensions come from its existing descriptors, not from
extra parameters or the source dimensions.

Neither body allocates, resizes, frees, retains, or replaces either grid.
Destination byte contents are overwritten; descriptors are unchanged on the
ordinary disjoint path. Source bytes are read without intentional modification.
All ownership and allocation remain with the caller. There is no allocation
failure branch inside these bodies. Caller allocation failure semantics are
outside the proven contract; see [GINT1](questions/grid-interpolation.md).

## 3. Coordinate mapping and fixed-point arithmetic

The following ordinary domain has fully derived results: valid, disjoint,
rectangular storage; all four counts in `1…32767`; valid 32-bit pointer
differences; and sufficient accessible byte storage. This is a sufficient
arithmetic/storage domain, not a discovered native size limit or an assertion
that larger grids are rejected.

Coordinates and weights are integers with 16 fractional bits. Define:

- outer step `p = floor(A × 65536 / C)`;
- inner step `q = floor(B × 65536 / D)`;
- initial outer coordinate `u₀ = floor(p / 2) − 32768`;
- initial inner coordinate `v₀ = floor(q / 2) − 32768`;
- destination `(i,j)` uses `u = u₀ + i×p`, `v = v₀ + j×q`.

The native sequence shifts the source count left 16 **in 32 bits**, sign-extends
that result for signed division, and uses `IDIV`; it does not multiply in 64-bit
precision before division (`0079EB8B–0079EBDD`, `0079F01B–0079F06D`). In the
ordinary domain these divisions are positive and truncate downward. Steps are
quantized once and then repeatedly added; the function does not recompute an
exact rational coordinate for each sample. Thus a non-divisible ratio can be
just below a mathematical half. Equal source/destination counts give step
65536 and initial coordinate zero. This is centre-based sampling with clamped
edges, not endpoint-to-endpoint scaling.

For completeness, the initial half-step instruction sequence subtracts the
step's sign mask before arithmetic shifting by one. For negative signed steps
it implements truncation toward zero when halving, rather than the positive
domain's floor. This is an observed instruction rule, not a supported negative
dimension contract. Coordinate adds, shifts, products and differences use
32-bit machine widths. No floating-point instructions or FPU rounding state
participate in either body.

## 4. Edge selection and interpolation order

### 4.1 Outer selection

For each destination outer element:

| Condition | Source elements selected | Outer fraction `f` |
|---|---|---|
| `u < 0` (signed comparison) | element 0 twice | 0 |
| `u ≥ (A−1)×65536` (signed comparison) | element `A−1` twice | 0 |
| otherwise | `a=floor(u/65536)` and `a+1` | low 16 bits of `u` |

The destination element pointer is fetched once per outer iteration. The source
element pointers are selected at that iteration, not copied into temporary
sample storage (`0079EC10–0079EC7C`, `0079F0A0–0079F10C`).

### 4.2 Inner selection and rounding

Define the integer interpolation operation, for `0 ≤ w ≤ 65535`:

`L(x,y,w) = x + floor(((y−x)×w + 32768) / 65536)`.

This is arithmetic-shift rounding with ties toward the greater integer,
including negative differences: `L(1,0,32768)=1`, `L(−1,0,32768)=0`.
It is not truncation toward zero of the weighted result or ties-to-even.

The interior inner branch uses an **unsigned** comparison of the raw 32-bit
coordinate `v` against `(B−1)×65536`. In the ordinary domain this is exactly the
test `0 ≤ v < (B−1)×65536` (`0079ECA1–0079ECA5`, `0079F131–0079F135`).

For interior `v`, let `b=floor(v/65536)`, `g=low16(v)`. First compute
`t₀=L(S[a,b],S[a,b+1],g)` and `t₁=L(S[a′,b],S[a′,b+1],g)`.
Then compute `r=L(t₀,t₁,f)`. Thus the inner axis is rounded separately for
each selected source element **before** interpolation along the outer axis.
This is a separable, two-stage rounded bilinear kernel, not one final rounding
of four exact weighted values (`0079ECA7–0079ECF5`, `0079F137–0079F185`).

For the non-interior inner branch, choose index 0 if arithmetic `v >> 16` is
negative, otherwise index `B−1`. Read that single byte from each selected outer
element, and compute `r=L(first,second,f)` directly. No invented zero sample is
used (`0079ED00–0079ED2B`, `0079F193–0079F1BE`).

### 4.3 Per-body input and store rules

At `0079EB40`, all interpolated sample loads zero-extend bytes. Store `r` as a
byte, with no bias (`0079ECA7–0079ECB4`, `0079ED0D–0079ED17`,
`0079ECFB`, `0079ED31`). The template's zero is not an out-of-bounds default.

At `0079EFD0`, sample loads sign-extend bytes, including the boundary branch's
first byte before its subtraction (`0079F137–0079F144`,
`0079F1A0–0079F1AA`). Add 128 **after** interpolation, in the final byte
register; store the low byte (`0079F18B–0079F18E`, `0079F1C4–0079F1C7`).
The output is an unsigned index representation: −128→0, −1→127, 0→128,
127→255. It is not a signed-byte grid that should be biased again on drawing.

Final addition/store is byte-width and there is no saturation branch. In the
ordinary domain the interpolated values remain within their input ranges, so
the unsigned output is already within 0…255. Intermediate differences can be
negative; the products plus 32768 fit signed 32 bits for byte values and these
fractions. Each inner iteration writes one byte, then advances `v` and the inner
index; each outer iteration advances `u` and the descriptor by 12
(`0079ED34–0079ED7D`, `0079F1CA–0079F213`).

## 5. Caller proof and limits

The unsigned caller starts at `007A2380`, the signed caller at `007A2740`.
Both check the interpolation flag at object offset `094C`, obtain city target
counts from virtual byte offsets `0174` and `0178` (slots 93 and 94), and take
the resampling path when **either** source count is below its corresponding
target. If both source counts are at least the target counts they bypass it
(`007A23AF–007A2403`, `007A276D–007A27CB`). This does not require both axes to
be coarser. The helpers themselves also support reduction in their ordinary
domain, regardless of this caller's admission policy.

The path constructs a local destination descriptor, passes source second and
destination first, calls the helper, then recursively uses the unsigned display
method with the temporary and city cell sizes. The signed path calls the
**unsigned** display method, consistent with the final 128 bias
(`007A2483–007A24BB`, `007A283A–007A2874`). Temporary storage is subsequently
released on this ordinary return path (`007A24C0–007A24F8`,
`007A2879–007A2893`). No original source-buffer mutation is required by this path.

There is a concrete rectangular-target caveat. The constructor at `007A1310`
creates the first target count's number of 12-byte descriptors, initialized as
empty byte vectors (`007A131B–007A1377`). Yet both caller sizing loops iterate
the **second** target count, resizing each visited element to that second count
(`007A2408–007A247D`, `007A27D0–007A2834`). For positive equal target counts this
supplies a square destination. For unequal target counts the loop does not
establish the rectangular storage precondition: it can leave elements empty or
visit descriptors beyond the allocation. This publication does not assert that
such city dimensions actually occur or that caller allocation failures recover.
Rectangular vectors below exercise the helper contract with already valid
destination storage, not this caller construction path.

The constructor's allocator target has an implausible exported class name;
its name is not accepted as a type contract. Only the descriptor/count setup
instructions are used here. Allocator/resize/failure ownership beyond this
ordinary preparation remains partial, not a new whole-owner specification.

## 6. Degenerate, invalid and aliasing cases

One source element on either axis is defined with valid storage: both edge
selections collapse to that element on that axis. A one-element destination
axis samples the centre according to the same formula; it is not necessarily
an average over all source elements. All-equal dimensions copy unsigned values
exactly and copy signed values with the final bias.

Empty grids are **not** established no-ops. Both bodies dereference the first
source and destination element while measuring lengths, before loop admission.
They divide by destination outer and inner counts before the outer-loop check
(`0079EB43–0079EBF7`, `0079EFD3–0079F087`). A zero divisor reaches native `IDIV`;
there is no local empty-grid return or error value. Zero source dimensions,
null/dangling pointers, non-rectangular storage and invalid descriptor ranges
are not validated. No portable return value or recovery is specified for them.

There is no alias check or input snapshot. Traversal is outer-increasing then
inner-increasing, with live source reads before each byte store; a later read
can observe an earlier store if buffers overlap. The mathematical matrix
contract assumes disjoint byte storage and descriptors. General overlap,
especially the signed body's in-place bias, cannot be advertised as safe copy
semantics. Descriptor overlap can also affect subsequent pointer fetches.

Larger counts and pointer arithmetic remain subject to 32-bit shift, wrap and
signed/unsigned branch effects, not a checked arbitrary-size resampler. The
ordinary-domain restriction above avoids those effects; it does not promote
unchecked inputs to supported behavior. Open boundaries are recorded in
[GINT1–GINT3](questions/grid-interpolation.md).

## 7. Independent literal vectors (proposed, not executed native)

These expectations are derived from the instructions and constants above.
They were not taken from engine code, portable outputs, native execution or a
whole-interpolation oracle. Shapes are `outer × inner`. Each destination is
already allocated with valid rectangular, disjoint storage.

| ID | Body / source | Destination shape | Expected destination bytes |
|---|---|---|---|
| G1 | unsigned `[[0,100],[200,255]]` | 4×4 | `[[0,25,75,100],[50,72,117,139],[150,167,200,216],[200,214,241,255]]` |
| G2 | unsigned `[[0,20,80],[100,160,200],[220,240,255]]` | 2×2 | `[[33,96],[198,236]]` |
| G3 | unsigned `[[0,20,80],[100,160,200]]` | 3×2 | `[[5,65],[60,127],[115,190]]` |
| G4 | unsigned `[[0,0],[0,1]]` | 1×1 | `[[1]]` |
| G5 | unsigned `[[0,255,7],[19,3,200]]` | 2×3 | `[[0,255,7],[19,3,200]]` |
| G6 | signed `[[-128,-1],[0,127]]` | 4×4 | `[[0,32,95,127],[32,64,127,159],[96,128,191,223],[128,160,223,255]]` |
| G7 | signed `[[-128,-1,0,127]]` | 1×4 | `[[0,127,128,255]]` |
| G8 | signed `[[-128,127]]` | 1×1 | `[[128]]` |
| G9 | signed `[[-1,0]]` | 1×1 | `[[128]]` |
| G10 | unsigned `[[7]]`; signed `[[-1]]` | 2×3 each | unsigned `[[7,7,7],[7,7,7]]`; signed `[[127,127,127],[127,127,127]]` |
| G11 | unsigned `[[0],[100],[255]]` | 1×4 | `[[100,100,100,100]]` |
| G12 | signed `[[-2,-1]]` | 1×1 | `[[127]]` |

Derivations and discriminators:

- **G1:** both steps are 32768; coordinates are
  `[-16384,16384,49152,81920]`. The first and last clamp. Inner interpolation
  produces `[0,25,75,100]` and `[200,214,241,255]`; outer fractions 16384 and
  49152 produce the two middle rows. In particular `75+round(166/4)=117`
  and `75+round(3×166/4)=200`. This rejects nearest sampling and endpoint scaling.
- **G2:** both steps are 98304, coordinates `[16384,114688]`. Inner results
  are `[5,65]`, `[115,190]`, `[225,251]`; outer interpolation at 1/4 and 3/4
  gives the literal result. For example `5+round(110/4)=33` and
  `190+round(3×61/4)=236`. It discriminates reduction from box averaging.
- **G3:** outer step is **43690**, initial coordinate **−10923**; outer
  coordinates are `[-10923,32767,76457]`. Inner step is 98304, giving
  `[5,65]` and `[115,190]`. At the middle outer coordinate,
  `L(65,190,32767)=127`, whereas using an exact half would give 128.
  This asymmetric case distinguishes axis order and repeated quantized steps.
- **G4:** both coordinates are 32768. Inner results are 0 and 1 (the latter
  rounds 1/2 upward); the outer result rounds their half to 1. One final
  rounding of the exact four-sample weight 1/4 would give 0.
- **G5/G7:** steps 65536 and coordinates equal integer source indices; the
  high edge uses the clamp branch. G7 additionally crosses both bias boundaries.
- **G6:** inner signed rows become `[-128,-96,-33,-1]` and
  `[0,32,95,127]`; their difference is 128 at each position. Outer quarters
  add 32 and 96. Only then is 128 added for all final stores. These include
  negative samples, signed zero, high signed samples and negative rounding.
- **G8/G9:** inner coordinate 32768. `−128+floor((255×32768+32768)/65536)=0`
  and `−1+floor((32768+32768)/65536)=0`; both final bytes are 128.
  They distinguish signed loads and upward ties from truncating each weighted
  delta. **G12** gives `−2+round(1/2)=−1`, hence 127 after bias; rounding the
  exact result −1.5 to even instead would give −2, hence 126.
- **G10:** the selected source on each axis always collapses to the sole
  sample, even though some initial coordinates are negative. **G11:** the
  outer coordinate is exactly 65536, selecting the middle of three elements;
  the inner axis collapses to its only sample. No empty-input vector is proposed.

## 8. Remaining scope

This publication supplies the ordinary two-body resampling algorithm, not a
MapView implementation or native conformance verdict. QV8's palette-ramp
algorithm and the other public MapView interface, rendering, lifetime and query
gaps remain open. No ABI export, coverage status, default behavior or host
callback contract is changed by this document.
