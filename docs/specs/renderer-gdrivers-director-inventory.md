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

`0x0087F0E0` fills the supplied descriptor's name pointer with **GDriverClass--OpenGL**, ORs
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

These contracts cover only the attributed support of the `0x7ACA35C6` product. They do not imply
completion of all operations on that product, or rename it to match misleading folded symbols.

### 5.1 Reusable buffer handles

`0x008EAF30` accepts only type 0 or 1; other types return handle zero without changing state. Handles
are **1-based array indices**, not pointers or serial numbers. Search from the first slot and use
the first inactive one. When live count is at least capacity, allocate room for capacity+16 records,
copy the old capacity's records, mark only the 16 new active flags false, increase capacity, free
old storage and install the new array before searching. Native record stride is 44 bytes, but a
portable representation need not use that layout. Allocation is not checked before copying/marking;
there is no null-allocation Boolean failure path or rollback. New records' other contents are not
blanket-zeroed (`0x008EAF5F–0x008EAFE2`).

On claiming a slot, increment live count, mark active, set its data pointer null and type to the
requested type, and assign a global 32-bit serial. If the next serial is zero, use 1; increment it
after assignment with wraparound. Serial generation is shared across objects. Initialize its origin
and dimensions to zero. Type 0 uses four bytes per pixel; type 1 uses two. The content-version word
and pitch are not initialized here; first allocation by the save operation below supplies them.
Return index+1. If the search finds no inactive slot, return zero, with no further growth/retry.
Evidence: `0x008EAFE2–0x008EB0B6`.

`0x008EAF00` considers a handle valid iff its unsigned value lies in 1..capacity and its active byte
is nonzero. It does not compare the serial, live count, data pointer or buffer size. Negative signed
inputs therefore fail the unsigned bound check (`0x008EAF04–0x008EAF20`). A released handle can
become valid again for a different buffer when the same slot is reused.

`0x008EB0C0` first invokes the object's virtual validity operation (table offset `0x20`). On false,
return false with no change. On true, mark inactive, free the stored data pointer, decrement live
count, and return true. It does not clear the freed pointer, serial, dimensions or type. A second
release normally fails validity and does not free again. `0x008EB110` visits slots in increasing
index order, invokes virtual release (table offset `0x14`) for each active slot, then frees the array,
sets array pointer and capacity zero and returns true. It does not explicitly reset live count;
ordinary release calls decrement it. Both validity/release results are low-byte Booleans. Evidence:
complete bodies `0x008EB0C0–0x008EB10F`, `0x008EB110–0x008EB172`.

### 5.2 Strided copy and dispatch endpoints

`0x008EAE90` copies a rectangle described by source, destination, byte width, source pitch,
destination pitch and height. If byte width or height is signed ≤0, return without invoking either
endpoint. If both pitches equal byte width, call the copy endpoint once with height-times-width
bytes. Otherwise call it once per row in increasing row order, advancing source/destination by
their own pitches. In either positive-size case, invoke the completion endpoint **once after all
copies**, not once per row. Do not add an overlap or negative-pitch rejection. All products and
address arithmetic are 32-bit. Native register arguments are width/source/destination in EAX/ECX/EDX;
this matters when reading the pseudocode, which omits width at some call sites.

The attributed entry `0x008DDFB0` is an unconditional indirect tail delegate through the current copy
endpoint pointer at `0x00B16104`; `0x008DDFC0` similarly delegates through `0x00B5CA90` for completion.
Neither validates its pointer or adds an independent copy/cleanup. The installed optimized endpoints
are separate support, not extra bodies added to this inventory. Evidence: `0x008EAE99–0x008EAEF7`
and the complete six-byte tail delegates.

### 5.3 Save frame rectangle into a buffer (`0x008EB180`, 447 bytes)

Use arguments `(handle, rx, ry, width, height, sx, sy)`, where r denotes destination buffer offset
and s source frame offset. Resolve the handle slot **without a validity/range check**. Require
rx,ry,sx,sy ≥0; require rx+width and sx+width ≤frame width, and ry+height and sy+height ≤frame height.
If buffer data already exists, also require rx+width/ry+height within its stored dimensions. Reject
failed comparisons with false before the frame-synchronization helper `0x008E0100`. Width/height
themselves are not required positive; sums and comparisons are signed 32-bit, with wrapped addition.

After those checks, invoke synchronization. For a null buffer pointer, allocate
`frame width × bytes-per-pixel × frame height + 8` bytes and immediately zero that entire size,
without checking allocation success. Initialize pitch to frame-width-times-pixel-size, version to
zero, origin to (0,0) and dimensions to full frame size. There is no resizing of an existing buffer.
Increment version before copying, including zero/negative copy sizes and unsupported stored types.

Type 0 copies four-byte pixels from the first frame plane; type 1 copies two-byte pixels from the
second plane. Pass source frame base plus sy-times-frame-pitch plus sx-times-pixel-size, destination
buffer base plus ry-times-buffer-pitch plus rx-times-pixel-size, byte width=width-times-pixel-size,
both pitches, and height to §5.2. Return true after the copy. Other stored types return false **after**
synchronization, lazy allocation if needed, and version increment. No validity call precedes these
side effects. Evidence: checks `0x008EB19B–0x008EB230`, allocation `0x008EB242–0x008EB290`,
version/type dispatch `0x008EB293–0x008EB2C0`, copy setup `0x008EB2C2–0x008EB33E`.

### 5.4 Restore buffer rectangle to frame (`0x008EB500`, 754 bytes)

Use arguments `(handle, sx, sy, width, height, dx, dy)`. Again resolve the handle directly without
validity checking. Require dx,dy ≥0, dx+width ≤frame width and dy+height ≤frame height. Compute
`u=dx−buffer origin x` and `v=dy−buffer origin y`; require u,v ≥0 and u+width/v+height within the
buffer's stored dimensions. These checks **do not validate the explicit source offsets sx/sy**.
Failed checks return false before synchronization or tile writes. There is no positive-size test.

Invoke synchronization. For type 0, first mark affected 16×16 frame tiles, in row-major order. Tile
columns run from arithmetic-shift-right(dx,4) through arithmetic-shift-right(dx+width−1,4), inclusive;
rows analogously use dy and dy+height−1. Store this 32-bit tag, using addition of the components:

`(((dx−sx) << 18) & 0x0FFC0000) + (((dy−sy) & 0x3FF) << 8) + (serial << 28) + (version & 0xFF)`.

Then clear the left tile column if dx has a nonzero low nibble, the right tile column if dx+width
has a nonzero low nibble, the top tile row if dy has a nonzero low nibble, and the bottom row if
dy+height has a nonzero low nibble. Each clear covers the same inclusive tile range; overlap is
allowed. Exact tile-boundary endpoints preserve the tag; partial boundary tiles become zero.
All arithmetic wraps at 32 bits; this is not a unique full-width serial/version encoding. Native
byte/shift/mask operations are at `0x008EB5C9–0x008EB621`; boundary loops
`0x008EB676–0x008EB72B`. Type 1 performs no tile-tag writes.

Source address is buffer base plus sy-times-buffer-pitch plus sx-times-pixel-size, even though the
fit tests used u/v. Destination is frame-plane base plus dy-times-frame-pitch plus dx-times-pixel-size.
Type 0 uses first plane/four-byte pixels; type 1 second plane/two-byte pixels. Delegate the copy to
§5.2 and return true. Another stored type returns true after synchronization without copying; it is
not the false-return rule of the save operation. A null data pointer is not independently rejected.
Evidence: `0x008EB500–0x008EB5A8`, full marking loops `0x008EB623–0x008EB674`, and copy/type/return
sequence `0x008EB731–0x008EB7F7`.

### 5.5 Software-frame capture (`0x008EB340`, 443 bytes)

Inputs are x,y,width,height and optional bitmap. Form the requested right/bottom before clamping
x/y to zero; cap right/bottom to the current frame dimensions and subtract clamped starts. No empty
or overflow rejection. Invoke synchronization **before** inspecting/creating the destination.
An initialized destination must report format 9 (format descriptor starts as words 3,16); otherwise
return null without releasing it. If accepted, independently limit capture width/height to its
dimensions. The native bitmap identity is used for these calls; misleading decompiler references
through another parameter are not part of the contract (`0x008EB3D9–0x008EB3FD`).

Without a bitmap, call GZCOM's factory slot with class `0xC470D325`, interface `0x20732180`, and the
bitmap output. Null output returns null. Unlike the OpenGL capture, this path does not null-check
the GZCOM accessor before calling through it. Initialize an uninitialized bitmap to capture dimensions
and format words 9,32. Initialization failure calls bitmap Release and returns null, including when
the bitmap was supplied by the caller. Lock with `0x8040`; lock failure also releases it and returns
null. An unsupported preexisting format does **not** take that Release path.

After successful lock, ask bitmap slot 35 (native offset `0x8C`) with frame pitch and capture height,
then pass its returned value to slot 34 (`0x88`) to obtain the destination address. Copy width×4
bytes per row from first-plane address `(x,y)` using frame pitch and that same pitch for destination,
with capture height rows; no RGB conversion or vertical reversal occurs here. Unlock with `0x8040`
and return the bitmap without adding a reference. Native call setup `0x008EB473–0x008EB4B2` supplies
height, frame pitch, frame pitch to the copy helper. Evidence: `0x008EB340–0x008EB4FA`.

### 5.6 Aggregate records and their storage helpers

This group manipulates two ordered sequences of opaque value records, A and B. Their native copy
sizes are 204 and 48 bytes, respectively. These sizes describe copying and allocation evidence,
not a mandated portable object layout. Copies are shallow: pointer-valued contents are copied as
values and receive no AddRef, deep clone or per-element destruction during sequence resizing.

| Body | Required behavior |
|---|---|
| `0x008DEC30` / `0x008DED50` | Reallocate A/B sequence to capacity `old size + max(old size, insertion count)`, using unsigned comparison for max. Copy prefix before the insertion position, then count copies of the supplied value. If the final input byte is zero, also copy the old suffix; nonzero omits that suffix. Free old backing storage, then install new begin/end/capacity. Copy widths are 51/12 words per record. No checked-allocation false result or rollback is provided. |
| `0x008DEF60` / `0x008DF0B0` | Insert repeated A/B values. Zero count does nothing. If unused capacity is insufficient, use the corresponding reallocator above with suffix retained. Otherwise snapshot the supplied record **before shifting**, preserving self-aliasing insertion. If tail length is greater than insertion count, move its last count records beyond old end, shift the remaining tail backward, then fill the gap. Otherwise append count-minus-tail copies, move the old tail, then fill its old positions. Maintain order and increase size by count. No element pointees are freed. |
| `0x008DF1F0` | Resize A to an unsigned requested count, with a supplied fill record. Shrink by moving end to the requested position, without freeing removed record pointees or shrinking capacity. Grow by inserting repeated fill records at old end; equality is a zero-count no-op. |
| `0x008DF260` | Resize A using a partially initialized default record: zero its key word, all nine owned entry pointers, the first eight associated value words, and a trailing flag byte. The ninth associated value word is not initialized by this template. Other record bytes have no defined initial value. This must not be specified as zeroing all 204 bytes. |
| `0x008DF2E0` | Initialize both sequences and additional aggregate counters empty/zero; resize A to one default record, then mark that first record's flag true and its two trailing working values zero. Return the aggregate identity. No allocation-failure check before touching the first record. |
| `0x008DEE80` | Destroy aggregate: in B order, free each record's first owned pointer; then in A order free nine entry pointers per record and clear each of those nine pointers. Free B backing storage, then A backing storage when nonnull. Do not reset sequence begin/end/capacity or promise this is safe to repeat. |

Evidence: native strides and shallow copy loops `0x008DECB1–0x008DECF2`,
`0x008DEDC0–0x008DEE23`; growth comparisons `0x008DEC56–0x008DEC68`,
`0x008DED73–0x008DED86`; insertion branches `0x008DEF9B–0x008DF0A2`,
`0x008DF0E0–0x008DF1E0`; resize `0x008DF215–0x008DF247`; partial template stores
`0x008DF26F–0x008DF2C5`; aggregate initialization `0x008DF2E6–0x008DF31D`; destruction
`0x008DEEB0–0x008DEF57`. Pointer/free order is behavior; no guessed graphics meaning is assigned
to record A/B or the template's unwritten scalar fields.

### 5.7 Interface and resource destruction order

`0x008DCBF0` adjusts an interface identity by −4 and tail-delegates QueryInterface to `0x008DC4C0`.
`0x008DCC50` adjusts by −12 and delegates deleting destruction to `0x008DCD60`.
`0x008DCC60` adjusts by −12 and delegates Release to `0x008DC550`;
`0x008DCC70` adjusts by −12 and delegates AddRef to `0x008DC540`. The last two targets' exported
`cSC4NetworkToolUI` names do not make these thunks UI operations. Inputs and delegated return are
preserved; no independent reference is added. Finalizer `0x008DC4A0` has the slot-80-then-delete(1),
return-zero rule in §3.5.

`0x008E0080` frees the two working-array pointers in their allocation order, without testing or
clearing them. `0x008DCC80` restores this branch's dispatch and performs the following ordered
cleanup: delegate branch stop to `0x008E1240`; free those two arrays; clear reusable buffers via
§5.1; free a nonnull trailing owned buffer; destroy the trailing collection of owned string buffers;
free a nonnull tile-tag buffer and another nonnull owned buffer; Release a nonnull held interface;
destroy the A/B aggregate; destroy the block pool (virtual allocation, descriptor storage and set);
then reference-base cleanup. The block-pool cleanup's misleading PathInfo/list export names do not
change the resources it actually releases. No failure result from stop is tested and no cleanup
failure is converted to a Boolean. This is distinct from the OpenGL destructor, which does not
invoke WGL stop. Evidence: complete `0x008DCC80–0x008DCD24`, supporting pool calls at
`0x008EBDFE`, `0x008EBE07`, `0x008EBE0E`, `0x008EBE1B`.

The deleting wrapper `0x008DCD60` always performs that destruction and frees outer object storage
only for flag bit 0, returning the canonical pointer. Its registered factory returns a different
interface identity, as §1 records; callers must not separately free that interior identity.

## 6. Independent resource/order vectors

| ID | Stimulus | Required distinguishing observation |
|---|---|---|
| GD51 | Create type 0, type 1, then type 2 in an empty object. | Handles 1,2,0; capacity grows to 16 on first create only. Pixel sizes 4 and 2; unsupported type does not change count. |
| GD52 | Release handle 1, validate it, then create again and validate the old number. | False after release, new handle 1, then old numeric handle valid again despite a different serial. Second release before reuse performs no free. |
| GD53 | Clear active handles 2,5,9, instrumenting virtual release. | Release order 2,5,9; array freed afterward; pointer/capacity zero; live count reaches zero through releases. |
| GD54 | Fail descriptor-array growth allocation from an empty object. | Invalid write while clearing new active flags; no handle-zero recovery or freeing old contents first. |
| GD55 | Copy byte width 8, height 3 with pitches (8,8), then (12,16), then width 0. | One 24-byte copy then completion; three 8-byte copies at their respective strides then one completion; no calls for width zero. |
| GD56 | Save an admissible zero-width rectangle into an unallocated type-0 handle. | Synchronize, allocate/zero full-frame storage plus 8, version increments; no copy/completion calls; true return. |
| GD57 | Fail that lazy data allocation. | Invalid write while zeroing allocation result; no false-return allocation branch or rollback. |
| GD58 | Restore type 0 at dx=dy=0, width=height=32, with serial=1, version=2, sx=sy=0. | Four tile tags become 0x10000002, then a strided copy; no edge clears. Type 1 changes no tags. |
| GD59 | Restore at dx=1,dy=0,width=31,height=32 with source sx=1,sy=0. | Initial four tags written; left column then cleared; right column remains tagged because right=32 is aligned. |
| GD60 | Set buffer origin (0,0), dimensions 64×64; restore dx=dy=0,size 16×16 but sx=−1. | Fit checks do not reject sx; copy uses source address one pixel before the buffer. No invented source-clamping rule. |
| GD61 | Inject unsupported stored type 2 and invoke save/restore with otherwise admissible zero-size rectangles. | Save synchronizes and increments version then false; restore synchronizes then true without copying. |
| GD62 | Software capture into caller-supplied bitmap: wrong initialized format, initialization failure, then lock failure. | Null without Release for wrong format; Release then null for either later failure. Distinguish the OpenGL path, which returns the bitmap on the latter failures. |
| GD63 | Insert two copies of an element already inside A, with old size 3/capacity 8; separately grow size 3 by 5 requiring allocation. | Snapshot makes self-alias insertion preserve original value and order; reallocated capacity is 8, not 10 or fixed +16. No pointee frees during insertion. |
| GD64 | Shrink A with allocated entry pointers, then destroy the aggregate. | Shrink only changes end; destruction visits remaining records, not removed ones. Removed pointees are not automatically freed by resize. |
| GD65 | Delete a second-factory object with flag 2 while recording cleanup. | Stop → working-array frees → reusable-buffer releases/array free → trailing owned resources → held-interface Release → aggregate destruction → pool destruction → reference-base cleanup; no outer free. |

## 7. Complete coverage-attributed checklist

Each row is one exported function body. **65 rows / 8,068 bytes**, with no deduplication by name and
no removal of neighbor-attributed entries. “Parent” means the linked director spec. All rows below
have a behavior contract; support outside these rows is cited separately and adds no bytes to this
denominator. The directly named 85 bytes are only the first two rows.

| Address | Bytes | Attribution rule | Body / completed contract |
|---|---:|---|---|
| `0x00466250` | 79 | name-medium | Director constructor; parent §2 |
| `0x004662A0` | 6 | name-low | Director ID; parent §2 |
| `0x0087D260` | 24 | reach | Optional callback Boolean; §3.1 |
| `0x0087D280` | 59 | reach | Selector callback; §3.1 |
| `0x0087D2C0` | 16 | reach | One-word forwarding; §3.1 |
| `0x0087D2D0` | 36 | reach | Five forwarded / seven consumed words; §3.1 |
| `0x0087D300` | 46 | reach | Seven-word forwarding; §3.1 |
| `0x0087D330` | 28 | reach | Lighting enable; §3.2 |
| `0x0087D350` | 47 | reach | Individual light enable; §3.2 |
| `0x0087D380` | 55 | reach | Ambient scalar permutation; §3.2 |
| `0x0087D3C0` | 101 | reach | Selected light property; §3.2 |
| `0x0087D430` | 88 | reach | Combined light pointer reuse; §3.2 |
| `0x0087D490` | 136 | reach | Selected material property; §3.2 |
| `0x0087D530` | 129 | reach | Combined material/shininess; §3.2 |
| `0x0087D750` | 29 | reach | Finalizer; §3.5 |
| `0x0087D880` | 541 | reach | Front-buffer capture; §3.3 |
| `0x0087DBC0` | 224 | reach | Name predicate; §3.4 |
| `0x0087DEE0` | 441 | reach | OpenGL product constructor; §1 |
| `0x0087E0A0` | 6 | free-neighbour | Class ID 0xC4554841; parent §2 |
| `0x0087E0B0` | 8 | reach | QI identity −4; §3.5 |
| `0x0087E0C0` | 8 | reach | Release identity −4; §3.5 |
| `0x0087E0D0` | 8 | reach | QI identity −8; §3.5 |
| `0x0087E0E0` | 8 | reach | Release identity −8; §3.5 |
| `0x0087E120` | 8 | reach | QI identity −16; §3.5 |
| `0x0087E130` | 8 | reach | Release identity −16; §3.5 |
| `0x0087E140` | 8 | reach | Delete identity −20; §3.5 |
| `0x0087E150` | 8 | reach | Release identity −20; §3.5 |
| `0x0087E160` | 105 | reach | OpenGL product destructor; §3.5 |
| `0x0087E1D0` | 35 | reach | First registered factory; §1 |
| `0x0087E200` | 30 | reach | OpenGL deleting wrapper; §3.5 |
| `0x0087F0E0` | 56 | reach | OpenGL descriptor fill; §3.4 |
| `0x0087F120` | 34 | reach | OpenGL callback descriptor; §3.4 |
| `0x0087FBD0` | 182 | reach | OpenGL mode callback; §3.4 |
| `0x008DA490` | 6 | reach | Class ID 0x7ACA35C6; parent §2 |
| `0x008DC430` | 109 | reach | Coefficient tables; §1.3 |
| `0x008DC4A0` | 29 | reach | Second-product finalizer; §3.5 |
| `0x008DC960` | 599 | reach | Second-product constructor; §1 |
| `0x008DCBF0` | 8 | reach | QI identity −4; §5.7 |
| `0x008DCC50` | 8 | reach | Delete identity −12; §5.7 |
| `0x008DCC60` | 8 | neighbour | Release identity −12; §5.7 |
| `0x008DCC70` | 8 | neighbour | AddRef identity −12; §5.7 |
| `0x008DCC80` | 165 | reach | Second-product destructor; §5.7 |
| `0x008DCD30` | 37 | reach | Second registered factory; §1 |
| `0x008DCD60` | 30 | reach | Second-product deleting wrapper; §5.7 |
| `0x008DDFB0` | 6 | reach | Copy endpoint tail delegate; §5.2 |
| `0x008DDFC0` | 6 | reach | Completion endpoint tail delegate; §5.2 |
| `0x008DEC30` | 274 | reach | A reallocation/insertion; §5.6 |
| `0x008DED50` | 297 | reach | B reallocation/insertion; §5.6 |
| `0x008DEE80` | 222 | reach | Aggregate destruction; §5.6 |
| `0x008DEF60` | 331 | neighbour | A insertion; §5.6 |
| `0x008DF0B0` | 317 | neighbour | B insertion; §5.6 |
| `0x008DF1F0` | 98 | neighbour | A resize with fill; §5.6 |
| `0x008DF260` | 127 | neighbour | A partial default template; §5.6 |
| `0x008DF2E0` | 72 | reach | Aggregate construction; §5.6 |
| `0x008E0080` | 26 | reach | Working-array frees; §5.7 |
| `0x008E00A0` | 88 | reach | Working-array allocation; §1.1 |
| `0x008EAE90` | 97 | reach | Strided copy; §5.2 |
| `0x008EAF00` | 47 | reach | Buffer-handle validation; §5.1 |
| `0x008EAF30` | 385 | reach | Buffer-handle allocation/reuse; §5.1 |
| `0x008EB0C0` | 80 | reach | Buffer-handle release; §5.1 |
| `0x008EB110` | 99 | reach | Ordered buffer clear; §5.1 |
| `0x008EB180` | 447 | reach | Frame-to-buffer save; §5.3 |
| `0x008EB340` | 443 | reach | Software-frame capture; §5.5 |
| `0x008EB500` | 754 | reach | Buffer-to-frame restore/tile tags; §5.4 |
| `0x008EC050` | 248 | reach | Block-pool construction; §1.2 |

The reach roots are genuine factory registrations, executable factory calls, and constructor/destructor
table references. Neighbor assignments are kept as measured, even where another named owner calls
the same body. No false-reference exclusion is proposed: none is needed to reconcile **8068 =
85 + 7983**. This checklist records this director's attributed coverage only; it confers no
whole-DX7-family or capped-driver completion credit. The parent spec's 17 primary and 11 secondary
director entries remain a separate dispatch checklist, not a competing function denominator.
