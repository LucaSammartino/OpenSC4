# Text tokens: the default detokenizer and the advice token methods (behaviour spec)

Scope: the owners `cSCStringDetokenizerDefault` (5 functions, 1,145 bytes), `fn:gAdviceDetokenizerFunction` (1,
2,393) and `fn:gNeighborDealAdviceDetokenizerFunction` (1, 515) as tools/coverage.py assigns them: the built-in
`#token#` values of the game's string detokenizer, and the two token methods the advisor system registers.

Related: rz-strings.md §6–§7 (the detokenizer base: registries, `ConvertToken`'s priority, the callback
convention `(token, output, context)`, case-insensitive names), lua-host.md §10.2 (the Lua expression tokens of the
advice method), advisors.md §8.5 and §12 (the advice text, `cISC4AdvisorSystem`, `cISC4Advice`,
`cISC4AdviceSubject` slots), city-situation.md (the situation manager slots), neighbors.md §5 (the deal), sl-library.md
§1 (the globals), application.md (the captions table, string `0x5377BE31`).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the three owners was read in the decompile and in the disassembly; the
token names are the static strings built at start-up (`0x00A70001`–`0x00A70301`, `0x00A708C1`–`0x00A70941`) and the
literals they copy. Written 2026-10-24 (re-tokens7).

All name comparisons below are case-insensitive (`IsEqual(name, false)`, rz-strings.md §4), on the token after the
base parser has trimmed it. Each method writes its result into the output string and returns true; "unresolved"
means the method returns false (the base then tries the next method, rz-strings.md §7).

## 1. The game's detokenizer (`cSCStringDetokenizerDefault`) *[compat]*

The object behind app slot 42 (`0x00B43CDC`): the base detokenizer (rz-strings.md §6) with vtable `0x00A86C68`
and five name strings (`+0x30`, `+0x44`, `+0x58`, `+0x6C`, `+0x80`), empty after the constructor (`0x0044B990`;
deleting destructor `0x0044CCD0`, the base's destructor).

`Init` (`0x00600D30`, the overridden slot 3) sets the names `City`, `Mayor`, `Year`, `Population`, `Region` and
registers each with `SetTokenReplacementMethod(name, 0x00600150, this)` (slot 6); true. The callback
(`0x00600150`) forwards to the handler (`0x005FFF50`), which compares the token with the five names in that order:

| Token | Output (unresolved when the object is missing) |
|---|---|
| `City` | the city's name (city slot 12 `GetCityName`; city global `0x00B43CEC`) |
| `Mayor` | the mayor's name (city slot 16) |
| `Year` | the simulation year (simulator `0x00B43D5C` slot 6, the five-pointer `GetSimDate`, first value), as a locale number (the language manager's formatter, slot 18, with the value sign-extended to 64 bits) |
| `Population` | the residential population (`0x00B43DA4` slot 16 `GetPopulation`), as a locale number |
| `Region` | the active region's name: app slot 34 `GetNation`, nation slot 8 `GetActiveRegion`, region slot 3 `GetName`, copied (output slot 13). A missing nation is unresolved; a missing active region is not checked *[quirk: a null dereference]* |
| other | unresolved |

## 2. The advice token method (`gAdviceDetokenizerFunction`, `0x00417AF0`) *[compat]*

Registered by the advisor system as an unknown-token method with the system as context (lua-host.md §10.2). The
token is copied and trimmed; the locale formatter is taken from the language manager (slot 38). Fixed names, in
the order tested:

| Token | Output |
|---|---|
| `link_id`, `sit_link_id` | `sc4://advice/%08x%08x` with the context advice's (system slot 20 `GetContextAdvice`) advisor ID (advice slot 11) and session ID (slot 13); both 0 without a context advice |
| `mission_target` | the city situation manager's (city slot 49) current target name (situation slot 16), copied |
| `num_targets` | `%d` of situation slot 11 |
| `cur_target_index` | `%d` of situation slot 10, plus 1 |
| `num_targets_remaining` | `%d` of situation slot 12 |
| `mission_mayor_rating` | `%+.0f` of situation slot 17 `GetSituationAura`, called with **both** outputs at one float, so the value the situation writes last is printed *[quirk]* |
| `mission_failure_mayor_rating` | `%+.0f` of the active situation's (situation slot 6) slot 49, called the same way |
| `mission_money` | `%+d` of situation slot 18 `GetSituationMoney` (an s64): only the low 32 bits are printed *[quirk]* |
| `mission_failure_money` | `%+d` of the active situation's slot 51 (an s64, low 32 bits) |
| `advisor` | the name (advisor slot 8) of the context advice's advisor (system slot 7 `GetAdvisor(advisor ID)`), fitted to a 200-pixel box of the default font `0x68963C4C` with the string `0x5377BE31` (an ellipsis) through the window manager's slot 35 and the font's slot 50 *[inferred: the name is shortened with "…" when too wide]* |
| `building_initial_cost`, `building_monthly_cost` | the context advice's event subject (advice slot 25) data1 (subject slot 9) or data2 (slot 11), as `SC4MakeMoneyString` |

Each mission token is unresolved without a city, a situation manager or (for the failure tokens) an active
situation; `link_id` is never unresolved; the other tokens are unresolved without the objects they read.

Any other token is a Lua expression (lua-host.md §10.2: the optional `X@` format prefix, `_dasd_=(…)`, the value
types). When the expression or any fixed-name token is unresolved, the output becomes
`%UNKNOWN TOKEN --<token>--%` (format `%%UNKNOWN TOKEN --%s--%%`, with the token after prefix removal) and the
method returns false; the base then replaces the output with the token (rz-strings.md §7 step 4).

## 3. The neighbour-deal token method (`gNeighborDealAdviceDetokenizerFunction`, `0x00421880`) *[compat]*

The token is copied and lower-cased. The context advice (system slot 20) must answer the neighbour-deal advice
interface `0xEA3848BA`; its event subject's object (advice slot 25, subject slot 8) must answer
`cISC4NeighborDeal` (`0x0773FEF1`). Otherwise, and for any other token, the method returns false and writes
nothing.

| Token | Output |
|---|---|
| `nd_city` | the name of the deal's other-side city (deal slot 7 `GetCityOnOtherSideOfConnection`, then that city's slot 33); unresolved when the deal has no such city |
| `nd_amount` | the deal's amount (deal slot 10), as a locale number |
| `nd_cost` | the deal's cost (deal slot 22, an s64), as `SC4MakeMoneyString` |

The interface references taken are released.

## 4. Function inventory

`cSCStringDetokenizerDefault`: `0x0044B990`, `0x0044CCD0`, `0x005FFF50`, `0x00600150`, `0x00600D30`.
`fn:gAdviceDetokenizerFunction`: `0x00417AF0`. `fn:gNeighborDealAdviceDetokenizerFunction`: `0x00421880`.
