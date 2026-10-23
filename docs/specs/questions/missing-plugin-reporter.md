# Missing-plugin reporter questions

## MPR1 — Allocation and provider failure outside the normal domain

Which, if any, native environmental guarantees prevent a null allocator cache, null 12-byte sentinel allocation, null eight-byte dialog allocation, or container allocation failure at the reporter's call sites? The observed instructions unconditionally dereference the allocator and sentinel, and the null-dialog allocation branch rejoins `DoDialog` with a null receiver. No graceful false return is supported for these edges. The spec records the exact last reporter action without claiming the foreign call returns or faults. Scope for a later authorized investigation: provider lifetime/allocation error channels or a targeted original execution, not the dialog's whole algorithm. References: `0x0048C783–0x0048C793`, `0x0048C82A–0x0048C844`, `0x0048C8DD–0x0048C8DF`.

## MPR2 — Reentrancy and external reference/cache lifetime

Which external guarantees keep cached service `0x00B207E4`, filter `R+0x10`, context `0x00B43DD8`, list links and borrowed property-holder/variant views valid across foreign calls? The reporter caches a returned service output then releases its temporary reference, rereads the cache at cleanup sites, releases occupants before advancing their list links, and permits the dialog before freeing list storage. No reporter-owned guard or extra cache AddRef was found in the two bodies. Native reentrancy, list invalidation, cache replacement and exception-unwind outcomes are unsupported. References: `0x0048C770–0x0048C783`, `0x0048C803–0x0048C80A`, `0x0048C84B–0x0048C8B5`, shared cleanup `0x00413E70–0x00413EC2`.

## MPR3 — Property adapter invalid views and foreign output-error policy

What guarantees a live holder from occupant slot `+0x18` and a live variant from property slot `+0x14`? What output writes, if any, does variant conversion `+0x64` perform when its AL is zero, and can a provider truthfully report success without writing all four output bytes? The intrinsic adapter tests only the property-lookup pointer, and the reporter branches only on conversion AL; its stack output is not initialized per lookup. The normal contract requires a valid full 32-bit output on success, ignores it on failure, and makes no invalid-pointer result claim. References: `0x0048C7C5–0x0048C7FE`, `0x005FD480–0x005FD4AD`. No foreign property conversion algorithm is authorized by this question alone.
