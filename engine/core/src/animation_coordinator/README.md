# cSC4AnimationCoordinator portable owner

Source: `docs/specs/animation-coordinator.md` §§1–6. No timeline, play/pause,
loop, frame scheduler or `cS3DAnimationCoordinator` contract is involved.

The owner stores the initialized flag, owned foreign service/subscription leases,
the two filter predicates, zoom 4/minimum 2, a visible block rectangle, square
grid of occupant sets, registered count and enabled flag initially true. Each
block's count is its set cardinality. All eligibility and registration decisions
are Rust code. Foreign host operations supply data, power slot 31 and tick manager
add/remove primitives. No host method makes coordinator decisions.

## Published method/helper inventory

These are all **19 function references** in the supplied public owner spec.
One is the model-maker creation context; two are destructor references. The
remaining 16 describe portable behavior at varying detail. They are not a
complete 45-entry attribution catalogue, and Rust method counts do not substitute
for that catalogue. Addresses below are spec identifiers, never bindings.

| Public spec reference | Portable implementation | Residual |
|---|---|---|
| `0x00447750` maker creates owner (§1) | `AnimationCoordinator::new` models published defaults | Maker function is foreign context; constructor ABI/catalogue AC9 |
| `0x00447B20` Init (§1.1) | `init`, filters, service/subscription leases, zoom, square grid, visible blocks | Native filter/interface lifetime AC6; failure AC7; global-byte trigger AC8 |
| `0x00447900` Shutdown (§1.1) | `shutdown`, unsubscribe, release services, empty sets/count | Extra native target teardown/refcounts AC6 |
| `0x00445FA0` animatable filter (§1.2) | `Occupant::animatable` | No invented filter callback/lifetime |
| `0x00446000` shown filter (§1.2) | `Occupant::shown_animatable` | Spec publishes predicate, not all call sites |
| `0x00446070` occupant block (§2) | `occupant_block`, position / 256, clamp | Edge/non-finite/empty-city AC5 |
| `0x00446890` SetOccupantAnimation (§2) | `set_occupant_animation`, sets/counts/gates | Movement/filter changes AC8; callback ordering AC7 |
| `0x00446290` may animate (§2) | `may_animate`, power property/slot 31 | **Budget predicate AC2 unimplemented**, explicit error |
| `0x00446150` activation/removal (§2) | `activate`, target query, add/remove 0xC0, flag 0x20 | Native target ownership AC6 and errors/reentry AC7 |
| `0x00445F10` changed-rectangle blocks (§3) | `refresh_rectangle` on normalized block bounds | **Native power rectangle conversion AC5 unimplemented** |
| `0x00446510` block reevaluation (§3) | `refresh_block` | Native iteration/reentry AC7 |
| `0x004465D0` occupant reevaluation (§3) | `reevaluate_occupant` | Budget AC2; target registration/movement AC8 |
| `0x004465A0` zoom/rotation update (§3) | `do_message` ZoomRotation branch | Native payload/return AC8 |
| `0x00446760` all-block refresh (§3) | `refresh_all` | Opaque message retains its raw identity (AC1); iteration AC7 |
| `0x004469B0` leaving/entering blocks (§3) | `move_visible_rectangle` difference selection | Rectangle bounds AC5; ordering AC7 |
| `0x00446D20` edge block reevaluation (§3) | `move_visible_rectangle` refreshes differences only | Reentry/order AC7 |
| `0x00446EB0` DoMessage slot 3 (§§1,3) | `do_message`, all five typed variants plus unknown | **Native message decoding/return ABI unimplemented**, AC5/AC8 |
| `0x00447A00` destructor body (§1) | Rust field Drop frees owned leases and grid | **Native lifetime/refcount contract unimplemented**, AC6 |
| `0x00447B00` deleting destructor slot 4 (§1) | Rust allocation/lifetime only | **Native deleting-destructor semantics unimplemented**, AC6/AC9 |

The enabling-byte threshold is implemented by `refresh_animation_enabled` with
the literal `byte >= 2` comparison and no inferred meaning. Its invocation sites
are AC8. `0x231BBF91` performs exactly the published all-block reevaluation; it
is not silently redefined as a settings notification.

## Explicit limits and choices

AC1/AC2 are preserved. AC3/AC4 were withdrawn because the coordinator's original
task supplied the wrong owner and fictional requirements; they are not contract
gaps and are never reused. Genuine new questions are AC5–AC9.

- AC2: budget-dependent eligibility returns `UnspecifiedBudgetTest(id)` without
  changing the animation flag. Other gates may prove animation must stop without
  evaluating this unknown predicate. There is no delegated budget eligibility test.
- AC5: half-open block bounds, floor lower/ceil upper world overlap, finite
  positions clamped to grid, invalid geometry rejected, empty grid registration
  rejected. Power messages accept explicitly normalized block rectangles only.
- AC6: portable RAII leases release only their own acquisitions. Occupant IDs are
  borrowed identities, not reference-counted native pointers. Shutdown does not
  invent additional tick removals or flag clearing. Native reference counts and
  deletion are unimplemented. Tests prove portable lease balance only.
- AC7: failed Init drops acquired leases and permits retry; repeat Init is a
  no-op. Membership precedes callbacks; manager success precedes flag writes.
  Missing targets/failed manager calls return errors with flags preserved.
  Refresh stops at first error, visits row-major blocks and sorted occupant IDs,
  and scrolling processes leaving blocks before entering blocks. State may be
  partially updated on error. Exclusive Rust mutations do not model native reentry.
- AC8: budget notifications do not register occupants. Set on/off operates on the
  occupant's current block. Global-byte refresh is explicit; message payloads are
  typed inputs, without a native decoder. No meaning is assigned to opaque bits.
- AC9: **45 functions / 7726 bytes** is the coordinator-supplied target. Only 19
  references are published here, including creation context and destructors.
  Per-function residual byte accounting and remaining entries are unavailable.

No whole-owner, ABI, shadow, native-reference or replacement promotion is claimed.
Native binding must resolve these contracts before safely using borrowed IDs or
attaching this core to the game. Immutable getters contain no RefCell; a future
wrapper supplies synchronization for the game's second-thread getters.

## Validation

Core Rust tests contain literal filter, position, visibility, zoom, power, raw
byte, flag and scroll vectors. Failure/order/reference cases explicitly name
their provisional question. Language-neutral executable scenarios are in
`engine/conformance/vectors/animation_coordinator_*.json`; their format extension
is documented in `engine/conformance/animation-coordinator.md`. No expected
decision is generated by calling the coordinator under test.
