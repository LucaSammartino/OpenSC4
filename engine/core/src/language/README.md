# Language core

The existing utility and manager implement language-utility.md and
strings-language.md. The string and base detokenizer modules implement
rz-strings.md without adding ABI exports or game bindings.

RzString owns counted bytes and an allocation terminator. Its default allocation
is eight bytes. Copies are independent and begin with reference count zero.
Release returns the existing Released disposition; the owning adapter must drop
the object on Destroy, including at count zero. The native layout, native
allocator and cdecl varargs remain adapter responsibilities.

DetokenizerDispatch preserves virtual calls from Detokenize to PerformDetokenize
and from the base parser to ConvertToken. Subclasses can use the public
perform_detokenize function with themselves as receiver. detokenize_in_place
models aliased native arguments. ResourceStrings only retrieves/releases foreign
interfaces; it does not implement a resource manager or invent locale policy.
ReplacementMethod carries an explicit unsigned callback identity and opaque
32-bit context. Its Rust callable is retained for safe invocation; this does
not imply AddRef/Release of the native context.

Unsupported results identify existing open questions. RZS1 registry mutation and
recursive invocation during callbacks are outside this non-reentrant API; the
implementation makes no snapshot-registration promise. RZS2 non-ASCII case
conversion/collation is not replaced by Unicode or ANSI approximation. RZS3
allocation/overlap/invalid pointer edges remain outside valid independent slices;
ordinary edit range errors are returned distinctly. RZS4 native split variants
return Unsupported. RZS5 formatter edge cases return Unsupported without changing
the receiver. Ordinary receiver identity is preserved by edit/assignment returns.

Sprintf parses typed arguments in core, builds output separately and assigns
only on completion. Integer conversion, glyph-based field padding and valid
multibyte string precision are implemented here. The exact float/pointer digit
conversion is not stated by §5; FormatPrimitives exposes that boundary pending
RZS6. Its test double verifies conversion selection and padding, not native
float/pointer parity. Unvectored integer zero/precision/alternate combinations use
the conventional literal interpretation, identified in code and RZS6. No
whole-format host printf substitution is used.

## Owner closure

Every §9 vector is an executable core unit test in rz_tests.rs; test names keep
RZ01–RZ20 and DT01–DT19 identifiers. “Not assigned” identifies sections about
the other owner rather than claiming an additional implementation.

| Owner | Spec section | Code item / boundary | Covering test |
|---|---|---|---|
| cRZString | §1 identity, state, lifetime | RzString, StringSource, query_interface_into, add_ref, release; native layout stays foreign; open: RZS3 failure/overlap | rz01_default_storage_and_lifetime, rz02_independent_counted_copy_and_buffer, dt17_query_failure_preserves_sentinel |
| cRZString | §2 inventory and dispatch | Public string methods, StringSource counted copy, assign receiver return; no new native vtable | string_inventory_wrappers_and_range_errors, rz06_replace_old_count_and_identity, rz18_sprintf_assignment_and_identity |
| cRZString | §3 assignment and edits | from_char, set_c_string, copy, resize, append, insert, replace, erase, strcat, strncpy; open: RZS3 invalid/overlapping inputs | rz02–rz07, string_inventory_wrappers_and_range_errors |
| cRZString | §4 equality, ordering, search | is_equal, compare_to, compare_string, find/rfind and object overloads, begins_with/ends_with; open: RZS2 non-ASCII folding | rz08–rz13, rz20_prefix_suffix_counted, open_boundaries_are_explicit_and_do_not_fabricate_results |
| cRZString | §5 transforms and formatting | trim, mid/left/right, split_token, make_lower/upper, sprintf; open: RZS2 non-ASCII mapping, RZS4 other split variants, RZS5 formatter edges, RZS6 numeric conversion details | rz14–rz19, settled_format_syntax_modifiers_and_multibyte_precision, formatting_numeric_primitives_are_unpadded_and_receiver_assigned_last, open_boundaries_are_explicit_and_do_not_fabricate_results |
| cRZString | §6 detokenizer lifetime | Not assigned | — |
| cRZString | §7 detokenizer registry | Not assigned | — |
| cRZString | §8 detokenizer parsing/resource IO | Not assigned | — |
| cRZString | §9 conformance | All RZ01–RZ20 plus both-owner DT17 | rz01–rz20, dt17_query_failure_preserves_sentinel |
| cRZString | §10 open boundaries | RZS2–RZS5 tracked explicitly; additional §5 gap filed as RZS6 | open_boundaries_are_explicit_and_do_not_fabricate_results |
| cSCStringDetokenizer | §1 string lifetime | Not assigned | — |
| cSCStringDetokenizer | §2 string inventory | No subscriptions, rendering, persistence or async contract | Base synchronous API only |
| cSCStringDetokenizer | §3 string edits | Not assigned | — |
| cSCStringDetokenizer | §4 string comparisons | Not assigned | — |
| cSCStringDetokenizer | §5 string transforms | Not assigned | — |
| cSCStringDetokenizer | §6 identity, state, lifetime, dispatch | Detokenizer, query_interface_into, add_ref/release, init/shutdown, HTML byte, DetokenizerDispatch; native layout stays foreign | dt01_base_literals_and_lifecycle, dt17_query_failure_preserves_sentinel, virtual_parser_converter_and_resource_dispatch |
| cSCStringDetokenizer | §7 registration and replacement | set_token_replacement_value/method, add_unknown_token_replacement_method, convert_token, key_equal; open: RZS1 callback mutation/reentrancy, RZS2 non-ASCII registry collation | dt02–dt03, dt07–dt13, registration_copies_names_and_values |
| cSCStringDetokenizer | §8 parsing and resource IO | perform_detokenize, detokenize_in_place, detokenize_resource, ResourceStrings | dt04–dt06, dt13–dt16, dt18–dt19, virtual_parser_converter_and_resource_dispatch |
| cSCStringDetokenizer | §9 conformance | All DT01–DT19 | dt01–dt19 |
| cSCStringDetokenizer | §10 open boundaries | open: RZS1 reentrancy and RZS2 non-ASCII collation | API excludes reentrant mutation; explicit Unsupported for non-ASCII registration |
