# Bounded graphics director core

Sources: `docs/specs/renderer-gdrivers-director.md` and its attributed inventory.
This pure core has no C ABI, plugin, runtime/default, game/shadow, or replacement wiring.
It claims no completion of nSGLDX7, nRZSimGL, cGZBufferSimGLCache, RD3–RD5, or the third
factory product. The module implements the attributed algorithms, with the explicit
specification boundaries below.

`Host` supplies primitive memory, COM, GL/WGL, and service operations. Its observation
hook only records events. The actual director, construction, capture, buffer, pool,
coefficient, collection, and cleanup algorithms execute here. Third-product creation,
primary driver QI/refcount operations, mode/device helpers, optimized copy endpoints
and general allocator internals remain foreign. `Products` is one process scope for
shared serials/coefficient tables. Addresses are wrapping 32-bit identity tokens.

`Fault` safely represents native unchecked access/division or a specification boundary;
it is not a native Boolean false return. Earlier effects and intentional leaks survive.
Unknown words/bytes use `Option` or known-bit masks. Inactive buffer contents carry no
initialization promise. There is no foreign cleanup in Rust `Drop`; cleanup is explicit
to preserve the specified call/free order. Invalid identities, use after destruction,
cycles and concurrent/reentrant collection mutation have no additional guarantee.

## Independent proof checklist

GD1–GD65 reproduce the paper stimuli. GD66–GD80 add public helper/boundary tests.
Eight JSON vector files contain 80 fixed expectations, written before execution.
The conformance fixture poisons newly allocated memory and supplies only primitive
results. Call-order predicates compare recorded calls against literal spec orders.
The negative-control test checks that a wrong expectation is rejected. No old renderer
or implementation is an output oracle.

Exactly 65 attributed bodies / 8,068 bytes are mapped below. This table does not add
transitive completion credit for foreign dependencies.

| Body | Bytes | Operation | Independent proof |
|---|---:|---|---|
| 00466250 | 79 | Director::new | GD1,2,11,25 |
| 004662A0 | 6 | Director::id | GD2 |
| 0087D260 | 24 | extension_bool | GD66 |
| 0087D280 | 59 | extension_selector | GD39 |
| 0087D2C0 | 16 | extension_one | GD66 |
| 0087D2D0 | 36 | extension_five | GD40,66 |
| 0087D300 | 46 | extension_seven | GD66 |
| 0087D330 | 28 | lighting | GD67 |
| 0087D350 | 47 | light_enabled | GD67 |
| 0087D380 | 55 | ambient | GD41 |
| 0087D3C0 | 101 | light | GD67 |
| 0087D430 | 88 | combined_light | GD42 |
| 0087D490 | 136 | material | GD67 |
| 0087D530 | 129 | combined_material | GD43,67 |
| 0087D750 | 29 | OpenGl::finalize | GD50 |
| 0087D880 | 541 | capture::front | GD44–46,72 |
| 0087DBC0 | 224 | OpenGl::name_predicate | GD47 |
| 0087DEE0 | 441 | OpenGl::new | GD32,47,68 |
| 0087E0A0 | 6 | OpenGl::class_id | GD68 |
| 0087E0B0 | 8 | Products::thunk QI −4 | GD69 |
| 0087E0C0 | 8 | Products::thunk Release −4 | GD69 |
| 0087E0D0 | 8 | Products::thunk QI −8 | GD69 |
| 0087E0E0 | 8 | Products::thunk Release −8 | GD69 |
| 0087E120 | 8 | Products::thunk QI −16 | GD69 |
| 0087E130 | 8 | Products::thunk Release −16 | GD69 |
| 0087E140 | 8 | Products::thunk delete −20 | GD74 |
| 0087E150 | 8 | Products::thunk Release −20 | GD69 |
| 0087E160 | 105 | OpenGl::destroy | GD50 |
| 0087E1D0 | 35 | Products::factory | GD7–9,12,32,33 |
| 0087E200 | 30 | OpenGl::delete | GD50 |
| 0087F0E0 | 56 | OpenGl::descriptor | GD68 |
| 0087F120 | 34 | OpenGl::callback_descriptor | GD68 |
| 0087FBD0 | 182 | OpenGl::mode | GD48,49 |
| 008DA490 | 6 | Software::class_id | GD74 |
| 008DC430 | 109 | Coefficients::regenerate | GD38 |
| 008DC4A0 | 29 | Software::finalize | GD74 |
| 008DC960 | 599 | Software::new | GD32,34–38 |
| 008DCBF0 | 8 | Products::thunk QI −4 | GD69 |
| 008DCC50 | 8 | Products::thunk delete −12 | GD74 |
| 008DCC60 | 8 | Products::thunk Release −12 | GD69 |
| 008DCC70 | 8 | Products::thunk AddRef −12 | GD69 |
| 008DCC80 | 165 | Software::destroy | GD65 |
| 008DCD30 | 37 | Products::factory | GD7,32,33 |
| 008DCD60 | 30 | Software::delete | GD65 |
| 008DDFB0 | 6 | copy_endpoint | GD55,60,72 |
| 008DDFC0 | 6 | completion_endpoint | GD55 |
| 008DEC30 | 274 | Sequence<ARecord>::reallocate | GD63,70 |
| 008DED50 | 297 | Sequence<BRecord>::reallocate | GD70 |
| 008DEE80 | 222 | Aggregate::destroy | GD64,65,78 |
| 008DEF60 | 331 | Sequence<ARecord>::insert | GD63,64 |
| 008DF0B0 | 317 | Sequence<BRecord>::insert | GD70 |
| 008DF1F0 | 98 | Sequence<ARecord>::resize | GD64 |
| 008DF260 | 127 | ARecord::partial_default / resize_default | GD32,64,70 |
| 008DF2E0 | 72 | Aggregate::new | GD32,64 |
| 008E0080 | 26 | WorkingArrays::free | GD65 |
| 008E00A0 | 88 | WorkingArrays::allocate | GD34,35 |
| 008EAE90 | 97 | strided_copy | GD55,56,72,75 |
| 008EAF00 | 47 | Buffers::valid | GD52 |
| 008EAF30 | 385 | Buffers::create | GD51,52,54,71 |
| 008EB0C0 | 80 | Buffers::release / release_via | GD52,53 |
| 008EB110 | 99 | Buffers::clear / clear_via | GD53,65 |
| 008EB180 | 447 | Buffers::save | GD56,57,61 |
| 008EB340 | 443 | capture::software | GD62,72 |
| 008EB500 | 754 | Buffers::restore | GD58–61,77 |
| 008EC050 | 248 | Pool::new | GD36,37,73 |

## Supporting dispatch

| Primary slots | Operations | Proof |
|---|---|---|
| 0 | query_interface | GD13,14,73 |
| 1,2,9,10 | add_ref, release, refs, remove_ref | GD15–17 |
| 3 | initialize | GD3–5,21–24 |
| 4,8 | primary_noop_one, primary_noop | GD20 |
| 5 | enumerate with same callback/context to children | GD11,26 |
| 6 | get_class, kind 1/2 and invalid kind | GD6–10,12,25,27,73 |
| 7 | can_unload | GD18,19 |
| 11,12 | framework, service | GD4,5,22,23 |
| 13 | id | GD2 |
| 14 | library_path | GD21,73 |
| 15 | heap_allocated_size | GD31 |
| 16 | delete / destroy_base | GD28 |

Secondary 0–2 share primary QI/refcounts; 3–10 use secondary_callback (GD20).
Singleton access/exit: GD1/29. Child flattening/initialization: GD24/30. Unsigned
collision/head order and duplicates: GD25/71. String length-before-data/copy and
embedded zero bytes: GD4/21/22. There is no specified unregister public operation;
destruction removes registrations without releasing driver products or children (GD28/29).

## Explicit specification boundaries

- Question GD1: general registry rehash chain order/table beyond the 193-bucket regime is
  absent. Growth reports a specification-boundary fault, not a native false result.
- GI1: no matrix shape or aggregate placement among constructor subobjects is given.
  Initial matrix words are zero without an invented shape. Pool-before-aggregate-before-
  working-arrays follows the literal prose; only the known-clear state bit is exposed.
- GI2: general pool descriptor/set allocator policy and virtual release flags are not
  given. Logical descriptor order and heap/virtual ownership are preserved without a
  guessed native container allocator. Optional foreign backing identities support cleanup.

These questions constrain equivalence claims; they do not authorize inferred behavior.
