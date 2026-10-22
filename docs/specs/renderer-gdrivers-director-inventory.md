# Graphics director: attributed factories and helpers

This is part of [the graphics-director specification](renderer-gdrivers-director.md). Scope is the
unchanged **65-body / 8,068-byte** coverage-attributed set, not just the 85 directly named bytes.
Addresses identify evidence and distinguish folded/misnamed operations; they do not prescribe native
object layouts. Rendering behavior outside this exact set, especially the capped DX7 driver, is not
claimed. Vectors are independent paper requirements derived from primary native control flow.

## 1. Factory construction and allocation failures

For class `0xC4554841`, the registered factory `0x0087E1D0` requests 420 bytes, calls constructor
`0x0087DEE0` only after successful allocation, and returns the constructed object's driver-interface
identity. For class `0x7ACA35C6`, `0x008DCD30` requests 1,048 bytes, passes the Boolean value false to
`0x008DC960`, and likewise returns its driver-interface identity. Either wrapper returns null after
an allocation failure, without calling the constructor. These are separate allocations on each call;
they neither reuse the director singleton nor initialize a device. The returned interface is an
interior identity of the allocated object, not a separate allocation (native adjustments 12 and 8).
Neither wrapper adds a reference. The director's subsequent QI/AddRef/Release sequence is specified
in the parent §4, including the null-result dereference. Evidence: both complete factory bodies,
especially `0x0087E1D0–0x0087E1F2` and `0x008DCD30–0x008DCD54`.

The first constructor installs the OpenGL implementation's interface identities, starts with no
bound external resources, empty mode/working collections and zeroed matrix storage, and copies the
name **Maxis 3D GDriver OpenGL 1.1**. Its 13-word selected-mode record is initially all zero except
its first word, which is `0xFFFFFFFF`. Eight light-state records begin with their two stored values
and two flags zero. Three scalar values start at binary32 `1.0`, a fourth at binary32 `0.5`; the
constructor performs no floating arithmetic. The later operations in §3 identify these states;
unwritten storage is not promised to be zero. It does not call OpenGL or create a window/context.
Evidence: `0x0087DEE8–0x0087DF38`, `0x0087DF8A`, `0x0087DFC8–0x0087DFF5`,
`0x0087E003–0x0087E098`.

The second constructor installs its own identities, preserves its input Boolean (the registered
factory supplies false), and initializes its resource handles/working collections empty. It constructs
the block pool below with block size 1,024 and requested count 128; initializes its temporary string
and working storage; clears a 13-word selected-mode record then sets its first word to `0xFFFFFFFF`;
allocates the two working arrays below; regenerates the coefficient tables; and finally clears bit
`0x2000` in the stored state word. This is construction, not a successful device-start report. Internal
allocation failures are not rolled back or translated into a factory Boolean failure. Evidence:
`0x008DC960–0x008DCBB6`, calls at `0x008DC9BF`, `0x008DCAD8`, `0x008DCB93`, `0x008DCB98`.

### 1.1 Working-array allocation (`0x008E00A0`, 88 bytes)

First request **90,112 bytes** for 1,024 records. Only each record's final two 32-bit words are
initialized to zero; other record contents are unspecified. On failure the stored record pointer is
null and construction continues. Then request **5,376 bytes**, store its result, and unconditionally
zero all 1,344 words. A null result of this second allocation is dereferenced: there is no safe false
return, no cleanup of the first allocation, and no record-pointer failure check at this point.
Evidence: `0x008E00A2–0x008E00F3`; first record stride is 88 bytes at `0x008E00CD`.

### 1.2 Block pool construction (`0x008EC050`, 248 bytes)

Create an empty identifier set (initial bucket request 100), empty block-descriptor collection and
zero pool counters. Round requested block size upward to a multiple of 32. Multiply that size by the
requested count using 32-bit arithmetic, round the product upward to a multiple of 4,096, then divide
by the rounded block size using signed division to obtain the actual count. Reserve/commit the
resulting count-times-size storage through the native virtual-allocation call with allocation flag
`0x1000` and protection `0x40`. For each index in increasing order, append a descriptor containing
the base plus index-times-size, zero, and the index. The registered construction request yields
size **1,024**, count **128**, total **131,072 bytes**. Allocation failure is not checked before
descriptors are built: a null base still produces addresses zero, 1,024, 2,048, and so on; this body
does not access those addresses. Invalid input such as zero size has no validation before division.
Evidence: `0x008EC095–0x008EC0D7`, `0x008EC0F0–0x008EC140`; no widened multiplication or floating
rounding is used. These failure rules concern this helper, not a promise that the complete object
survives later use of a failed pool.

### 1.3 Coefficient tables (`0x008DC430`, 109 bytes)

Every call regenerates 64 global entries in index order, without allocation or a guard. For index
`i=0..63`, let `q = floor(i × 65536 / 252)` and `r = 16384 − q`. One table stores the packed word
`q | (q << 16)` and then `q`; the companion stores `r | (r << 16)` and then
`r | 0x40000000`. All are 32-bit words. For indices 0 and 63, `(q,r)` is `(0,16384)` and
`(16384,0)` respectively. These are integer tables, not floating interpolants; they are rewritten
even if another instance already initialized them. Evidence: signed multiply/add/shift quotient
sequence `0x008DC442–0x008DC453`, stores `0x008DC463–0x008DC47E`, loop `0x008DC481–0x008DC496`.

## 2. Independent construction/failure vectors

| ID | Stimulus | Required distinguishing observation |
|---|---|---|
| GD32 | Allocate either registered factory twice with successful allocators. | Two constructor calls and distinct storage; no device/context creation or extra AddRef inside either wrapper. |
| GD33 | Fail the factory's outer allocation. | Null result, constructor not entered. The director's unsafe use of that null remains the parent §4 rule. |
| GD34 | Let the second factory's 90,112-byte request fail, but its 5,376-byte request succeed. | First pointer remains null; second storage is entirely zeroed; constructor does not return false or undo earlier allocations. |
| GD35 | Let the 90,112-byte request succeed and the 5,376-byte request fail. | Invalid write through null during zeroing, without a cleanup/Boolean failure path. |
| GD36 | Construct the pool with size 1,025 and count 2. | Rounded block size 1,056; rounded total 4,096; signed quotient count 3; allocation size 3,168. Three ordered descriptors, not four and not two. |
| GD37 | Fail virtual allocation for the registered 1,024/128 pool request. | 128 descriptors still appended; bases 0,1024,…,130048; no failure return from this helper. |
| GD38 | Regenerate coefficient tables after modifying their contents. | Both tables overwritten in full. Index 1 has q=260, r=16124; endpoint 63 has q=16384, r=0. |

## 3. First registered factory: OpenGL branch

These operations belong to the `0xC4554841` factory product reached by the director. This does not
extend the separate capped DX7 driver's specification.

### 3.1 Extension dispatch

| Body | Contract |
|---|---|
| `0x0087D260` | Return false if the global callback at `0x00B541D0` is null; otherwise invoke it without arguments and normalize its zero/nonzero result to false/true. |
| `0x0087D280` | If callback `0x00B541C0` is absent, return zero. Selectors 0,1,2 tail-delegate with callback argument 1,2,3 respectively; preserve its return. Other selectors return zero without a callback. |
| `0x0087D2C0` | Unconditionally call callback `0x00B541C4` with the supplied argument, then return Boolean true regardless of its result. |
| `0x0087D2D0` | Unconditionally call callback `0x00B541C8` with the first five supplied 32-bit arguments, in original order, then return Boolean true. The native entry consumes seven argument words, although only five are forwarded. |
| `0x0087D300` | Unconditionally call callback `0x00B541CC` with all seven supplied words in original order, then return Boolean true. |

The last three operations do not guard a null callback or report its failure. Callback installation
and the callbacks' own contracts are outside this attributed set. Evidence: complete bodies at the
listed addresses; `0x0087D2EF–0x0087D2F1` proves the five/seven distinction, which pseudocode hides.

### 3.2 Lighting and material forwarding

All calls below take effect immediately through OpenGL, without storing a deferred driver command
or inspecting an OpenGL error. Numeric GL names are provided as unambiguous constants.

| Body | Contract |
|---|---|
| `0x0087D330` | Enable lighting (`0x0B50`) for any nonzero input byte; disable it for zero. |
| `0x0087D350` | Enable/disable light `0x4000 + index` by the same byte rule; no index bound check. |
| `0x0087D380` | For scalar arguments `(a,b,c,d)`, call light-model ambient (`0x0B53`) with binary32 words **(a,c,b,d)**, not the input order. No arithmetic or clamping. |
| `0x0087D3C0` | Selector 0,1,2 forwards the supplied vector pointer to ambient/diffuse/specular (`0x1200/0x1201/0x1202`) of light `0x4000+index`. Other selectors do nothing, including negative selectors. |
| `0x0087D430` | Combined ambient/diffuse/specular setter: test each of the three supplied pointers independently, in that order. For each nonnull pointer call the corresponding property, but **all three calls pass the first pointer**. Thus nonnull diffuse with null ambient passes null to GL. |
| `0x0087D490` | Material selectors 0..4 map to ambient, diffuse, specular, emission, shininess (`0x1200,0x1201,0x1202,0x1600,0x1601`). Always target front (`0x0404`), forward the supplied pointer without a null check; other unsigned selector values do nothing. |
| `0x0087D530` | Combined material setter: nonnull ambient/diffuse/specular/emission pointers are passed to their own properties, in that order, targeting front. The fifth argument is binary32 shininess: forward its address for values ≥0, including −0 and +infinity; skip negative values and NaN. No upper bound clamp. |

Evidence: pointer reuse at `0x0087D445`, `0x0087D45C`, `0x0087D473`; scalar permutation
`0x0087D383–0x0087D3AB`; native jump-table words at `0x0087D518`; binary32 comparison
`0x0087D58B–0x0087D59A` against the zero word at `0x00A81054`.

### 3.3 Front-buffer capture (`0x0087D880`, 541 bytes)

Inputs are x, y, width, height, and an optional destination bitmap interface. Query GL viewport
`0x0BA2`. Compute the requested right/bottom before clamping x/y to at least zero; clamp right/bottom
to viewport **width/height**, ignoring its x/y origin. Subtract the clamped start to obtain the capture
dimensions. Use signed 32-bit comparisons and wrapped 32-bit sums/products. There is no empty-area,
negative-size or overflow check before requesting `width × height × 3` scratch bytes. Failure of
this allocation returns the original destination unchanged without a GL read or bitmap creation.

After allocation, call the graphics-service accessor. For a supplied initialized bitmap, request its
format using a descriptor initially containing words 3 and 16; require returned format 9. A different
format returns null **without freeing the scratch allocation**. For accepted format, independently
limit the read width/height to the bitmap's width/height. The scratch size remains the original size.
The native instructions use the destination bitmap for both dimension calls; the decompiler's
apparent calls through the scratch buffer are erroneous (`0x0087D948–0x0087D96C`).

If no destination was supplied, request one from the service via its output argument. If still null,
free scratch and return null. A supplied uninitialized bitmap proceeds directly to capture.
Zero the entire original scratch size, select front buffer `0x0405`, and set pack alignment
`0x0D05` to 1. Read RGB (`0x1907`), unsigned byte (`0x1401`) at
`(clamped x, viewport height − clamped y − final capture height)` with the final dimensions.
There is no restoration of the previous read buffer or pack alignment and no GL success check.

Next, if the bitmap is uninitialized, initialize it with capture width/height and format words 9,32.
If initialization fails, free scratch and return the bitmap pointer anyway. Otherwise lock with
flags `0x8080`; a failed lock likewise returns the bitmap after freeing scratch. Only successful
locks copy pixels: source RGB triples become `0xFFRRGGBB`, x increases from zero, and destination y
decreases from height−1 to zero. Unlock using the same flags even for a nonpositive copy dimension,
then free scratch and return the bitmap. No additional reference or destination Release occurs here.
Evidence: `0x0087D897–0x0087D8F8`, early leak `0x0087D935–0x0087D945`, service request
`0x0087D976–0x0087D991`, GL calls `0x0087D9A9–0x0087D9E2`, copy/cleanup
`0x0087D9E8–0x0087DAA2`.

### 3.4 Metadata, name predicate and mode callback

`0x0087F0E0` fills the supplied descriptor's name pointer with **GDriverClass: OpenGL**, ORs
`0x06080000` into its existing flags, sets the two dimension words at descriptor offsets 16 and 20
to **600 and 800** respectively, and records the object's canonical identity in a global used by
this branch. It returns true. Other descriptor contents are preserved. `0x0087F120` writes only
descriptor word 0 = `0x23`, word 1 = callback `0x0087EE10`, and word 9 = the same name pointer;
it returns true. These are descriptor fields, not a prescribed driver object layout.
Evidence: complete native bodies `0x0087F0E0–0x0087F117`, `0x0087F120–0x0087F141`.

`0x0087DBC0` copies the stored name, lowercases the copy, and performs two case-sensitive sequence
searches. Success requires length ≥6 and first occurrence of **nvidia** at offset zero, followed by
length ≥7 and first occurrence of **geforce** also at offset zero. Thus ordinary immutable names
cannot pass both conditions; do not replace this with a plausible “contains NVIDIA or GeForce”
rule. The temporary is freed on either Boolean return; the stored name is unchanged. Native strings
are at `0x00AC36A8` and `0x00AC36A0`; the two zero-offset tests are `0x0087DC34–0x0087DC38` and
`0x0087DC5C–0x0087DC60`.

The mode callback `0x0087FBD0` takes a window, mode selector, and optional auxiliary value. For
selector −1 with an already-unselected record (first word −1), return true without any work.
Otherwise selector −1 invokes the branch reset helper `0x0087E220`, then a no-op shared entry
`0x00407990`, obtains the **current** WGL DC and context, detaches the context with
`wglMakeCurrent(currentDC,null)`, deletes that current context, releases the DC against the supplied
window, and sets the selected-record first word to −1. Results of all these operations are ignored.
The no-op must not be interpreted as a memory clear just because its export name is `_memset`.

For any selector other than −1, store the auxiliary value only when nonzero; associate the canonical
object identity with the window using `SetWindowLongA` index −21; invoke the driver interface's
slot-65 operation with the selector and selected-record output; invoke `0x0087EE80` with the canonical
identity; then invoke `0x0087FA00` with the selector. Return true regardless of helper/OS results.
The delegated mode enumeration/creation helpers are separate named/shared owners, not newly claimed
here. Evidence: `0x0087FBD7–0x0087FC85`, no-op instruction at `0x00407990`.

### 3.5 Interface forwarding and destruction

The following are identity-adjusting tail delegates. Arguments, output locations and the delegated
return remain unchanged; no independent reference operation is added by a thunk.

| Entry | Identity adjustment | Delegated operation |
|---|---:|---|
| `0x0087E0B0` | −4 | primary QueryInterface `0x0087D670` |
| `0x0087E0C0` | −4 | primary Release `0x0087D740` |
| `0x0087E0D0` | −8 | primary QueryInterface |
| `0x0087E0E0` | −8 | primary Release |
| `0x0087E120` | −16 | primary QueryInterface |
| `0x0087E130` | −16 | primary Release |
| `0x0087E140` | −20 | deleting destructor `0x0087E200` |
| `0x0087E150` | −20 | primary Release |

The reference-count finalizer `0x0087D750` first calls driver slot 80 (native table offset `0x140`),
then invokes the object's deleting-destructor identity with flag 1, then returns zero. It does not
return either delegated value. This order also applies to the second branch's finalizer
`0x008DC4A0`, with that object's own identities.

`0x0087E160` restores the branch's interface dispatch, destroys its trailing collection by freeing
each nonnull element-owned string buffer in order and then the collection storage, frees its own
nonnull name buffer and another nonnull owned buffer, then performs reference-base cleanup. No WGL
stop/detach operation occurs in this destructor. The deleting wrapper `0x0087E200` always performs
that destruction; it frees the object's outer allocation only when flag bit 0 is set, and returns
the original canonical pointer. Other flag bits do not affect freeing. Evidence: both complete
bodies and the supporting collection destructor `0x004638B0–0x004638E8`.

## 4. Additional independent vectors

| ID | Stimulus | Required distinguishing observation |
|---|---|---|
| GD39 | Extension selector 1, then selector 3, with a callback returning 27. | First callback receives 2 and its return is preserved; second makes no call and returns zero. |
| GD40 | Call five-argument forwarding entry with seven distinct input words; callback returns false. | Only first five words forwarded in order; outer Boolean result true. |
| GD41 | Ambient scalar setter receives (1,2,3,4). | GL receives vector (1,3,2,4), with unchanged binary32 words. |
| GD42 | Combined light setter receives distinct nonnull A,D,S pointers. Repeat with A=null, D nonnull, S=null. | First log uses A for all three properties; second log makes just the diffuse call with null data. |
| GD43 | Material shininess inputs −1, −0, NaN, +infinity. | Skip, forward, skip, forward respectively; no clamp to 128. |
| GD44 | Capture at x=−2,y=1,width=5,height=2 with viewport (10,20,8,6), initialized destination 8×6 format 9. | Read at (0,3), size 3×2; ignore viewport origin. Destination row 1 receives the first RGB row and row 0 the second, both with alpha 255. |
| GD45 | Capture with initialized destination format other than 9. | Null return and leaked scratch allocation; no GL read, no destination release. |
| GD46 | Fail scratch allocation; separately fail bitmap initialization after a successful read. | First returns original destination without read; second returns bitmap pointer after freeing scratch, despite failed initialization. |
| GD47 | Name inputs NVIDIA, GeForce, NVIDIA GeForce, and mixed-case variants. | Predicate false for each; no modification of stored names. |
| GD48 | Stop mode twice, logging current WGL context/DC and helper calls. | First stop resets, detaches/deletes current context, releases supplied window's DC, marks −1; second makes no such calls. OS failure does not change true return. |
| GD49 | Start mode with auxiliary value zero, then nonzero, with all delegates reporting failure. | Zero preserves previous auxiliary state; nonzero replaces it. Window association, record output, two helpers occur in that order; both calls return true. |
| GD50 | Delete with flags 0,2,1 on separate fresh objects; also invoke finalizer. | All delete calls destroy contents; outer storage freed only for flag 1. Finalizer logs slot 80 before deletion and returns zero. |

## 5. Second registered factory: remaining support

Tracing and per-body checklist continue below.
