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
multibyte string precision are implemented here. RZS6 §5.1 integer prefix and
precision rules, sign/field ordering and eight-uppercase-digit pointers are
implemented without host printf. NumericPrimitives implements floating decimal
rounding, fixed/exponential/general layout, the 512 precision cap, zero-sign
normalization and the legacy infinity/NaN text (including letter rounding).

Finite nonzero extraction uses the default LegacyDecimal provider in
legacy_decimal.rs, implementing §5.1.1 (RZS7): exact widening including
subnormals, adjusted stored scaling constants in base-eight order, truncated
16-bit limb products with local ties-to-even rounding, one-step correction,
88-bit fixed-point extraction and decimal half-up guard rounding. The provider
remains injectable. All numeric conformance tests use the shipped default;
RZ41–RZ50 assert raw eighteen digits, rounded intermediate and final output.

## Owner closure

Every §9 vector is an executable core unit test in rz_tests.rs or numeric_tests.rs;
test names keep RZ01–RZ50 and DT01–DT19 identifiers. “Not assigned” identifies sections about
the other owner rather than claiming an additional implementation.

| Owner | Spec section | Code item / boundary | Covering test |
|---|---|---|---|
| cRZString | §1 identity, state, lifetime | RzString, StringSource, query_interface_into, add_ref, release; native layout stays foreign; open: RZS3 failure/overlap | rz01_default_storage_and_lifetime, rz02_independent_counted_copy_and_buffer, dt17_query_failure_preserves_sentinel |
| cRZString | §2 inventory and dispatch | Public string methods, StringSource counted copy, assign receiver return; no new native vtable | string_inventory_wrappers_and_range_errors, rz06_replace_old_count_and_identity, rz18_sprintf_assignment_and_identity |
| cRZString | §3 assignment and edits | from_char, set_c_string, copy, resize, append, insert, replace, erase, strcat, strncpy; open: RZS3 invalid/overlapping inputs | rz02–rz07, string_inventory_wrappers_and_range_errors |
| cRZString | §4 equality, ordering, search | is_equal, compare_to, compare_string, find/rfind and object overloads, begins_with/ends_with; open: RZS2 non-ASCII folding | rz08–rz13, rz20_prefix_suffix_counted, open_boundaries_are_explicit_and_do_not_fabricate_results |
| cRZString | §5 transforms and formatting | trim, mid/left/right, split_token, make_lower/upper, sprintf; open: RZS2 non-ASCII mapping, RZS4 other split variants, RZS5 formatter edges | rz14–rz19, settled_format_syntax_modifiers_and_multibyte_precision, formatting_numeric_primitives_are_unpadded_and_receiver_assigned_last, open_boundaries_are_explicit_and_do_not_fabricate_results |
| cRZString | §5.1 numeric conversion and assembly | sprintf sign/prefix/padding and integer digits; NumericPrimitives decimal rounding/layout, pointer digits, zero and special values | rz21–rz40, rounding_before_first_significant_digit_and_exponent_carry, default_legacy_provider_formats_finite_and_zero |
| cRZString | §5.1.1 finite decimal primitive (RZS7 answered) | legacy_decimal::convert, Extended::multiply, extract; default LegacyDecimal for () | rz41–rz50: raw digits, intermediate and output; rz21–rz40 use same default provider |
| cRZString | §5.1.2 signs, special values and owner field assembly | NumericPrimitives special markers and zero normalization; sprintf sign/field assembly | rz24–rz25, rz35–rz39, rz46_legacy_truncation_three_quarters_and_sign |
| cRZString | §6 detokenizer lifetime | Not assigned | — |
| cRZString | §7 detokenizer registry | Not assigned | — |
| cRZString | §8 detokenizer parsing/resource IO | Not assigned | — |
| cRZString | §9 conformance | All RZ01–RZ50 plus both-owner DT17; numeric vectors exercise shipped extraction | rz01–rz50, dt17_query_failure_preserves_sentinel |
| cRZString | §10 open boundaries | RZS2–RZS5 tracked explicitly; numeric §5.1 and §5.1.1 closed | open_boundaries_are_explicit_and_do_not_fabricate_results, invalid_decimal_provider_is_rejected_without_assignment |
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

The inherited utility/manager coverage is mapped below to its own specifications.
LocaleHost supplies the explicitly delegated platform operations; these rows
describe the core contract and unit coverage, not measured Windows locale parity.

| Owner | Spec section | Code item / boundary | Covering test |
|---|---|---|---|
| cRZLanguageUtility | language-utility §1, §1.1 identity and output | LanguageUtility lifecycle, counted byte outputs | utility_identity_and_lifetime, case_and_compare |
| cRZLanguageUtility | language-utility §2 IDs and attributes | language/country state and attribute/character-set queries | attribute_queries, character_sets |
| cRZLanguageUtility | language-utility §3.1–§3.2 currency/separators | currency_symbol, separators and flags | currency_symbol, separators_and_currency_flags |
| cRZLanguageUtility | language-utility §4.1–§4.3 grouping/integer formatting | grouped, make_number_string, make_money_string | number_strings, money_strings |
| cRZLanguageUtility | language-utility §4.4–§4.5 floating formatting | make_formatted_money_string, make_formatted_number_string; explicit LocaleHost printf | formatted_money_strings, formatted_number_string_uses_host_printf |
| cRZLanguageUtility | language-utility §4.6–§4.7 time/date | make_time_string, make_date_string | time_strings, date_strings |
| cRZLanguageUtility | language-utility §4.8 case/comparison | convert_case, compare_strings; explicit LocaleHost ANSI operations | case_and_compare |
| cRZLanguageUtility | language-utility §5.1–§5.4 names | names tables, day_name/abbr_day_name/month_name/abbr_month_name | day_and_month_names |
| cRZLanguageUtility | language-utility §6 open questions | open: Q1 external currency-flag writer, Q2 callers, Q3 runtime negative-zero spelling; host policy stays explicit | currency_symbol, formatted_money_strings; no caller/Windows parity claim |
| cRZLanguageManager | strings-language §1, §4.1 identity/state | lang constants, LanguageManager, embedded LanguageUtility, query_interface | manager_identity_and_utilities |
| cRZLanguageManager | strings-language §4.2 switching/availability | set_current_language/country, available-set methods; LocaleHost locale changes | manager_switching, manager_available_lists |
| cRZLanguageManager | strings-language §4.3 system mapping | current_system_language, system_language_from_language, language_from_system_language | manager_system_language |
| cRZLanguageManager | strings-language §4.4–§4.5 names/codes/runtime names | language name, alias, directory, ISO639 and runtime-library lookups | manager_names_and_codes |
| cRZLanguageManager | strings-language §4.6 countries | country names, ISO3166 and language/country mappings | manager_countries |
| cRZLanguageManager | strings-language §4.7 reverse lookup | reverse and language/country lookup methods; specified safe choice for recursive native lookup | manager_reverse_lookups |
| cRZLanguageManager | strings-language §4.8 money | exchange_rate_for_locale, convert_money | manager_identity_and_utilities |
| cRZLanguage | named owner in brief | No separate behavior assigned by these specs; language IDs/state are implemented by lang, LanguageUtility and LanguageManager | attribute_queries, manager_identity_and_utilities |

strings-language §2 delegates encoding to string-encoding.md; §3 delegates to
language-utility.md. Its §9 explicitly leaves UTF-16/Thai helpers, advice
detokenization, caller selection and startup language outside the settled contract.
