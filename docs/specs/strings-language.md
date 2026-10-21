# Spec: strings and language (`cRZLanguageUtility`, language IDs, framework string encoding)

Status: **behaviour spec, partial**, clean-room input. This document describes *behaviour*, not code. It was written
by the analysis team from observing the original program and its data files. Implementers must work from this spec,
the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled or
disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words and tags (*[compat]*, *[choice]*, *[inferred]*, *[quirk]*) as in [foundations.md](foundations.md).
Written 2026-10-21 by the re-strings4 lane. Evidence: the Ghidra export (instruction bytes, jump tables, string data)
and the decomp/ pseudocode; every name table below was produced by walking the instructions of each function for
every language ID 0..0x2d and every index, and every string is quoted from the bytes stored in the program. Every
address is SimCity 4.exe GOG 1.1.641. No modern Unicode semantics are implied: strings are byte strings.

Outline: §1 language IDs; §2 framework encoding; §3 the language utility object; §4 calendar names; §5 currency and
number punctuation; §6 per-language flags; §7 case conversion and comparison; §8 not yet specified.

## 1. Language IDs

A language is a 32-bit ID. The English names the language manager reports (`GetLanguageEnglishName`, 00924798):

| ID | name | ID | name | ID | name | ID | name |
|---|---|---|---|---|---|---|---|
| 1 | English | 0x0c | Hebrew | 0x17 | Bulgarian | 0x22 | Turkish |
| 2 | UK English | 0x0d | Russian | 0x18 | Cyrillic | 0x23 | Brazilian Portuguese |
| 3 | French | 0x0e | Portuguese | 0x19 | Ukranian (sic) | 0x24 | Swiss French |
| 4 | German | 0x0f | Japanese | 0x1a | Czech | 0x25 | Canadian French |
| 5 | Italian | 0x10 | Polish | 0x1b | Greek | 0x26 | Belgian French |
| 6 | Spanish | 0x11 | Simplified Chinese | 0x1c | Hungarian | 0x27 | Swiss German |
| 7 | Dutch | 0x12 | Traditional Chinese | 0x1d | Icelandic | 0x28 | Swiss Italian |
| 8 | Danish | 0x13 | Thai | 0x1e | Romanian | 0x29 | Belgian Dutch |
| 9 | Swedish | 0x14 | Korean | 0x1f | Latin | 0x2a | Mexican Spanish |
| 0x0a | Norwegian | 0x15 | Hindi | 0x20 | Slovak | 0x2b | Tagalog |
| 0x0b | Finnish | 0x16 | Arabic | 0x21 | Albanian | 0x2c | Vietnamese |

The rest of the language manager (system-language mapping, ISO codes, country tables, directory names, switching)
is not specified yet (§8).

## 2. Framework string encoding

- The framework's current code page (00940da8) is the constant **UTF-8** in this build. Encoding codes used by the
  string converter (`ConvertStringEncoding`, 00910e18): 8 = UTF-8, 0x10 = UTF-16, 0xFFFFFFFE = "the framework
  encoding" (treated as 8), 0xFFFFFFFF = "the system ANSI code page". The converter's full behaviour is §8.
- Every name/symbol getter in §4 and §5 ends with a "convert UTF-8 to the destination format" step (00923d58). With
  the framework code page fixed at UTF-8 that step returns immediately: **the stored bytes are delivered unchanged**
  *[compat]*. (If the framework code page were neither UTF-8 nor UTF-16, the step would convert the string from
  UTF-8 into that code page through a 256-byte buffer; unreachable in this build.)
- *[quirk]* The tables are not uniformly UTF-8. Many entries are stored in a legacy single-byte code page (Latin-1 /
  cp1252 for French, German, Portuguese, Finnish, Swedish; cp1250 for Polish; cp1251 for Russian; cp874 for Thai) or
  are double-encoded. Because the conversion is a no-op those raw bytes reach the caller. This spec writes such bytes
  as `\xNN`; an implementation must return exactly those bytes, not a "corrected" UTF-8 form.

## 3. The language utility object (`cRZLanguageUtility`)

- Interface: answers QueryInterface for ID 0x1054B7D5 and for 1 (the base unknown); AddRef/Release are ordinary
  reference counting, Release destroying the object when the count would drop below 1 (00927c2a, 00927c4f).
- State: language ID (initially 1, English), a country ID (initially 0x36), a "local currency" flag (initially false)
  and a "group separator" string (initially the two bytes C2 A0, UTF-8 no-break space). Constructor 0092847c;
  `Init` (00928fdd) re-assigns the separator to C2 A0, runs the no-op conversion of §2 and returns true.
- `SetLanguage(id)` (009b2970) stores the ID with no validation; every query below reads the stored ID.
- Who sets the country and the currency flag is not specified yet (§8).

## 4. Calendar names

`GetDayName(day, out)` (00925a26), `GetAbbrDayName` (00925f69), `GetMonthName(month, out)` (009265b2),
`GetAbbrMonthName` (009270b2). Days are 1 = Monday .. 7 = Sunday; months 1 = January .. 12 = December.

Common rule for all four *[compat]*: if the (language, index) pair has a table entry, `out` is set to it. Otherwise
`out` is not touched. Then, if `out` is empty, it is set to `Untranslated` and the function returns **false**;
in every other case it returns **true**. So an unsupported language or an out-of-range index (0, 8+, 13+) leaves a
caller's non-empty string as it was and reports success *[quirk]*.

Languages with no entry in any of the four functions: 0, 0x15..0x22, 0x24..0x2d (and any larger ID). Below, "EN" is
the English row.

### 4.1 Day names

| languages | Mon .. Sun |
|---|---|
| 1, 2, 9, 0x0c, 0x0d, 0x10, 0x13 (EN) | Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday |
| 3 | Lundi, Mardi, Mercredi, Jeudi, Vendredi, Samedi, Dimanche |
| 4 | Montag, Dienstag, Mittwoch, Donnerstag, Freitag, Samstag, Sonntag |
| 5 | Lunedì, Martedì, Mercoledì, Giovedì, Venerdì, Sabato, Domenica (UTF-8) |
| 6 | Lunes, Martes, Miércoles, Jueves, Viernes, Sábado, Domingo (UTF-8) |
| 7 | Maandag, Dinsdag, Woensdag, Donderdag, Vrijdag, Zaterdag, Zondag |
| 8, 0x0a | Mandag, Tirsdag, Onsdag, Torsdag, Fredag, Lørdag, Søndag (UTF-8) |
| 0x0b | Maanantai, Tiistai, Keskiviikko, Torstai, Perjantai, Lauantai, Sunnuntai |
| 0x0e, 0x23 | Segunda-feira, `Ter\xE7a-feira`, Quarta-feira, Quinta-feira, Sexta-feira, `S\xE1bado`, Domingo |
| 0x0f | 月曜日 火曜日 水曜日 木曜日 金曜日 土曜日 日曜日 (UTF-8) |
| 0x11, 0x12 | 星期一 星期二 星期三 星期四 星期五 星期六 星期天 (UTF-8) |
| 0x14 | 월요일 화요일 수요일 목요일 금요일 토요일 일요일 (UTF-8) |

Swedish (9), Hebrew, Russian, Polish and Thai get the English names; Danish and Norwegian share one row; Portuguese
"Domingo" is the same stored string as Spanish.

### 4.2 Abbreviated day names

| languages | Mon .. Sun |
|---|---|
| 1, 2 | Mon, Tue, Wed, Thur, Fri, Sat, Sun |
| 3 | lun, mar, mer, jeu, ven, sam, dim |
| 4 | Mo, Di, Mi, Do, Fr, Sa, So |
| 5 | Lun, Mar, Mer, Gio, Ven, Sab, Dom |
| 6 | L, M, X, J, V, S, D |
| 7 | Ma, Di, Wo, Do, Vr, Za, Zo |
| 9 | `M\xE5E`, Tis, Ons, Tor, Fre, `L\xF6r`, `S\xF6n` |
| 0x0b | ma, ti, ke, to, pe, la, su |
| 0x0d | `\xCF\xED`, `\xC4\xF2`, `\xD1\xF0`, `\xD7\xF2`, `\xCF\xF2`, `\xD1\xE1`, `\xC4\xF1` (cp1251) |
| 0x0e, 0x23 | Seg, Ter, Qua, Qui, Sex, Sab, Dom |
| 0x0f | same strings as the full Japanese day names |
| 0x10 | Pn, Wt, `\xC2\x8Cr`, Cz, Pt, So, N |
| 0x11, 0x12 | same strings as the full Chinese day names |
| 0x13 | `\xA8`, `\xCD`, `\xBE`, `\xBE\xC4`, `\xC9`, `\xCA`, `\xCD\xD2` (cp874) |
| 0x14 | same strings as the full Korean day names |

*[quirk]* Unlike the full names, Danish (8), Norwegian (0x0a) and Hebrew (0x0c) have **no** abbreviated day names
(they fall to the common rule: `Untranslated`/false on an empty string). Swedish has its own row here although its
full names are English. The Polish Wednesday is the three bytes C2 8C 72 (a cp1250 "Ś" double-encoded).

### 4.3 Month names

| languages | Jan .. Dec |
|---|---|
| 1, 2, 0x0c, 0x0d, 0x13 (EN) | January, February, March, April, May, June, July, August, September, October, November, December |
| 3 | janvier, `F\xE9vrier`, mars, avril, mai, juin, juillet, `Ao\xFBt`, septembre, octobre, novembre, `D\xE9cembre` |
| 4 | Januar, Februar, `M\xE4rz`, April, Mai, Juni, Juli, August, September, Oktober, November, Dezember |
| 5 | Gennaio, Febbraio, Marzo, Aprile, Maggio, Giugno, Luglio, Agosto, Settembre, Ottobre, Novembre, Dicembre |
| 6 | Enero, Febrero, Marzo, Abril, Mayo, Junio, Julio, Agosto, Septiembre, Octubre, Noviembre, Diciembre |
| 7 | Januari, Februari, Maart, April, Mei, Juni, Juli, Augustus, September, Oktober, November, December |
| 8 | januar, februar, marts, april, maj, juni, juli, august, september, oktober, november, december |
| 9 | Januari, Februari, Mars, April, Maj, Juni, Juli, Augusti, September, Oktober, November, December |
| 0x0b | tamikuu, helmikuu, maaliskuu, huhtikuu, toukokuu, `kes\xE4kuu`, `hein\xE4kuu`, elokuu, syyskuu, lokakuu, marraskuu, joulukuu |
| 0x0e, 0x23 | Janeiro, Fevereiro, `Mar\xE7o`, Abril, Maio, Junho, Julho, Agosto, Setembro, Outubro, Novembro, Dezembro |
| 0x0f | "1 月", **none**, "3 月" .. "12 月" (number, ASCII space, 月 in UTF-8) |
| 0x10 | `Stycze\xF1`, Luty, Marzec, `Kwiecie\xF1`, Maj, Czerwiec, Lipiec, `Sierpie\xF1`, `Wrzesie\xF1`, Padziernik, Listopad, `Grudzie\xF1` |
| 0x11, 0x12 | "1月" .. "12月" (no space) |
| 0x14 | 1월, 이월, 3월, 4월, 오월, 유월, 칠월, 팔월, 9월, 10월, 11월, 십이월 |

*[quirk]* Japanese February has no full-name entry (the common rule applies), although "2 月" exists in the abbreviated
table. Norwegian (0x0a) has no month names at all. Strings that are the same text in two rows (for example "Maj",
"April", "May") may be one stored string; only the bytes matter.

### 4.4 Abbreviated month names

| languages | Jan .. Dec |
|---|---|
| 1, 2, 8, 0x0b, 0x0c, 0x0d, 0x13 | Jan, Feb, Mar, Apr, May, Jun, Jul, Aug, Sep, Oct, Nov, Dec |
| 3 | jan, `F\xE9v`, mar, Avr, mai, juin, juil, `Ao\xFBt`, sept, oct, nov, `D\xE9c` |
| 4 | Jan, Feb, `M\xE4r`, Apr, Mai, Jun, Jul, Aug, Sep, Okt, Nov, Dez |
| 5 | Gen, Feb, Mar, Apr, Mag, Giu, Lug, Ago, Set, Ott, Nov, Dic |
| 6 | En, Feb, Mar, Abr, Mayo, Jun, Jul, Ag, Sep, Oct, Nov, Dic |
| 7 | Jan, Feb, Mrt, Apr, Mei, Jun, Jul, Aug, Sep, Okt, Nov, Dec |
| 9 | Jan, Feb, Mar, Apr, Maj, Jun, Jul, Aug, Sep, Okt, Nov, Dec |
| 0x0e, 0x23 | Jan, Fev, Mar, Abr, Mai, Jun, Jul, Ago, Set, Out, Nov, Dez |
| 0x0f | "1 月" .. "12 月" (all twelve) |
| 0x10 | Sty, Lut, Mar, Kwi, Maj, Cze, Lip, Sie, Wrz, `Pa\x9F`, Lis, Gru |
| 0x11, 0x12 | "1月" .. "12月" |
| 0x14 | same strings as the full Korean month names |

Danish and Finnish use the English abbreviations; Norwegian again has none.

## 5. Currency and number punctuation

### 5.1 `GetCurrencySymbol(out)` (009279b0)

If the local-currency flag is false, the symbol is `$` for every language. Otherwise:

| languages | stored bytes | reads as |
|---|---|---|
| 1, 0x11, 0x12 | 24 | `$` |
| 2 | C2 A3 | £ |
| 3, 4, 5, 6, 7, 8, 0x0a, 0x0b, 0x0e, 0x1b, 0x26 | E2 2C 82 2C AC | *[quirk]* five bytes: the UTF-8 euro sign with commas between its bytes |
| 9, 0x1d | 6B 72 | kr |
| 0x0c | E2 82 AA | ₪ |
| 0x0d | D0 B1 | б |
| 0x0f | C2 A5 | ¥ |
| 0x10 | C5 82 | ł |
| 0x13 | E0 B8 BF | ฿ |
| 0x14 | 5C | `\` (the won sign in Korean code pages) |
| 0x1a | 4B C4 8D 7B | `Kč{` *[quirk]* trailing brace stored |
| 0x1c | 46 54 | FT |
| 0x1e | 4C 65 69 | Lei |
| 0x21 | 4C 65 6B | Lek |
| 0x22 | 54 4C | TL |
| 0x23 | 45 73 63 2E | Esc. |
| any other | (not set) | |

Then the common rule of §4 (empty → `Untranslated`, false; otherwise true).

### 5.2 Currency placement

- `DoesCurrencySymbolPrecedeAmount` (00927ae3) is true for 1, 2, 5, 7, 8, 0x0a, 0x0c, 0x0f, 0x11, 0x12, 0x13, 0x14,
  0x22, 0x23; false for every other ID.
- `IsSpaceBetweenCurrencySymbolAndAmount` (00927b2f) is false for 1, 2, 0x0a, 0x0c, 0x0f, 0x11, 0x12, 0x13, 0x14;
  true for every other ID (including unknown ones).

### 5.3 Separators

`GetDecimalSeparator(out)` (00927b7b) and `GetThousandSeparator(out)` (00927c66) always write `out` and return a flag:

| languages | decimal | thousand | returns |
|---|---|---|---|
| 1, 2, 0x0c, 0x0f, 0x11, 0x12, 0x13, 0x14 | `.` | `,` | true |
| 3, 4, 5, 6, 7, 8, 9, 0x0d, 0x0e, 0x23 | `,` | `.` | true |
| 0x0a, 0x0b, 0x10 | `,` | the group-separator string of §3 (C2 A0) | true |
| any other | `.` | `,` | **false** |

`MakeFormattedNumberString(value, out, format)` (00927bee) ignores the language: it formats the 64-bit floating value
with the caller's format string through the string object's printf-style formatter and returns true.

## 6. Per-language flags

| query (address) | true for | otherwise |
|---|---|---|
| `DoesLanguageUseCharacterSet(cs)` (0092580a), cs = 1252 | the 1252 list below | false |
| same, cs = 1250 | 0x10, 0x0d, 0x21, 0x1a, 0x1c, 0x1e, 0x20 | false |
| same, cs = 932 / 936 / 950 / 949 / 874 | 0x0f / 0x11 / 0x12 / 0x14 / 0x13 respectively | false |
| same, cs = 1251 | 0x0d, 0x18, 0x17 | false |
| same, cs = 1253 / 1254 / 1255 / 1256 | 0x1b / 0x22 / 0x0c / 0x16 | false |
| same, any other cs | | false |
| `GetCharacterSetFromLanguage` (009256f5) | the first of 1252, 1250, 874, 932, 936, 950, 949, 1251, 1253, 1254, 1255, 1256 for which the query above is true; 1252 if none | |
| `AreFirstAndLastNamesReversed` (0092599b) | 0x0f, 0x11, 0x12, 0x14 | false |
| `DoesLanguageUseInputMethodEditing` (009259b8) | 0x0f, 0x11, 0x12, 0x14 | false |
| `DoesLanguageUseMultiByteCharacters` (009259d6) | every ID except 1 and 2 (because the framework code page is UTF-8, §2) | false for 1, 2 |
| `GetLanguageRoadDrivingSide` (00927c17) | returns 0 for 2 (UK English) and 0x0f (Japanese), 1 for every other ID | |
| `GetMeasurementSystem` (00925991) | returns false when the country ID is 0x36, true otherwise (reads the country, not the language) | |

The 1252 list exactly: 1, 2, 3, 4, 5, 6, 7, 8, 9, 0x0a, 0x0b, 0x0e, 0x1c, 0x1d, 0x1e, 0x1f, 0x23, 0x24, 0x25, 0x26,
0x27, 0x28, 0x29, 0x2a. Because 1252 is tried first, Hungarian (0x1c) and Romanian (0x1e) report 1252 although
they also match 1250, and Russian (0x0d) reports **1250**, not 1251, because it is in the 1250 list *[quirk]*.
The selector also tries 1251 twice and finally 1257, which no language matches, so it cannot return 1257. *[inferred]*: road-side 0 means left-hand traffic and the 0x36 country is the United
States (imperial units); the values themselves are exact.

## 7. Case conversion and comparison

- `ConvertToLowerCase(in, out)` / `ConvertToUpperCase(in, out)` (00927e7e, 00927edf): convert `in` from the framework
  encoding to the system ANSI code page, change case of the converted bytes in place with the operating system's
  ANSI lower/upper-case routine for a byte count equal to the **current length of `out`** (not of the converted
  text) *[quirk]*, then convert the buffer back to the framework encoding into `out`. Implementations should pass
  the converted length when `out` already holds the same text, which is how the callers use it *[choice]*.
- `CompareStrings(a, b, caseSensitive)` (00927f40): convert both to the system ANSI code page, compare them with the
  operating system's locale comparison for the user default locale, ignoring case when `caseSensitive` is false.
  Returns -1, 0 or 1 (a < b, equal, a > b). *[quirk]* If the system comparison fails, the result is 0 ("equal").

## 8. Not yet specified

`MakeNumberString` (00929379), `MakeMoneyString` (0092901a), `MakeFormattedMoneyString` (00928c55), `MakeDateString`
(00928699), `MakeTimeString` (009284ba); the rest of `cRZLanguageManager`; `cRZString` (formatting, search, compare);
`ConvertStringEncoding` and the UTF-16 / Thai helpers; `cSCStringDetokenizer` and `gAdviceDetokenizerFunction`.
