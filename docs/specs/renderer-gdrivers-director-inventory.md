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

## 3. Remaining attributed contracts

Round-2 tracing in progress; the complete per-body checklist follows with the remaining contracts.
