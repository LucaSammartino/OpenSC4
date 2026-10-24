# Web and network framework (behaviour spec)

Status: behaviour spec, written 2026-10-21 (re-webnet4 lane). Every fact was read from the Windows executable
`bin/SimCity4.exe` (1.1.641, image base `0x00400000`) through the Ghidra export (`tools/re_query.py`, export-first:
the shared REA session had no binary open during this lane, so no fresh REA pseudocode was used) and the
per-class pseudocode; branch shapes that matter were checked against the instructions at the cited addresses. Method
names are the Mac build's; helper functions without a matched name are cited by address and described by role
*[inferred]*. Implementers work from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They
must **not** consult decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Scope. The family is the engine's own small web stack: a URL parser (`cRZURL`), an HTTP/1.x client
(`cRZHTTPClient`), an FTP client reached only from `cGZWWWSystem`, a cookie store (`cRZCookieManager`), a disk cache
(`NetworkFileCache`), sockets and name lookup (`cRZNet*`, `cRZNameLookup`) and the `cGZWWWSystem` job front end.
The game uses it for the in-game HTML documents (`cHTMLDocument` resolves links and images with `cRZURL::ChangeURL`).
This revision specifies **`cRZURL` completely** (section 2), **`cRZCookieManager` partly** (section 3), and
the **cGZWWWSystem front end** (section 4, added 2026-10-22 by re-web-largest4). The latter owner's reached
FTP bodies are included in its inventory but are not automatically covered by the front-end contract.
Section 5 adds the HTTP-client inventory, progress and notification contract; section 6 adds the
cache storage/IO contract, and section 7 the command-generator dispatch boundary (re-www-ww5,
2026-10-23). These are bounded partial contracts; open wire/parser/file questions are explicit.
The dedicated [native transport contracts](network-transport.md)
qualify socket/stream/lookup behavior separately; the owner-size row below is not behavior credit.
No live network request was made.

Tags: *[inferred]* = role named by behaviour, not by symbol. *[quirk]* = observed behaviour that looks unintended;
an implementation must reproduce it for byte-exact parity.

## 1. Owner sizes (tools/coverage.py --owners, 2026-10-21)

| Owner | Bytes | Specified here |
|---|---|---|
| cGZWWWSystem (includes reached FTP bodies) | 34049 | partial, section 4; body checklist controls credit |
| cRZHTTPClient | 15133 | partial, section 5 and linked lifetime contract |
| cRZURL | 9498 | **yes, section 2** |
| cRZCookieManager | 8137 | partly, section 3 (parse, add, lookup, eviction; file format and capacity open) |
| NetworkFileCache | 6699 | partial, section 6; persistence/admission residuals Q11 |
| cRZNetSocket, cRZNetStreamSocket, cRZNetCommandGenerator, cRZNameLookup | 11671 | transport contract linked; command-generator boundary section 7 |
| HTTPMessage | 957 | no |

## 2. cRZURL — URL parser and resolver

### 2.1 State

A URL object holds these values (all strings are byte strings, initially empty):

| Field | Meaning |
|---|---|
| allowRelative (bool) | false after the default constructor; **true** after the construct-from-string constructor (00821c60, writes byte +0 = 1). Copied by assignment. Decides how a string with no recognised scheme is read (2.3 step 6). |
| parsed (bool) | result of the last parse; false until a parse succeeds. Getters reparse while it is false. |
| type | URL type code, table 2.2 |
| full | the URL text as given (GetFullURL returns it unparsed) |
| scheme, user, password, server | components |
| port (unsigned 32) | 0 = none given |
| path, params, query, fragment | components; params, query and fragment keep their leading `;`, `?`, `#` |

Lazy parse. GetURLType, GetScheme, GetServer, GetAbsolutePath (the path), IsURLRelative and
MakeURLStringFromComponents first check `parsed`; when it is false they parse a copy of `full` (2.3) and then read the
field. A URL that does not parse is therefore reparsed on every getter call, with the same result.

**Method inventory (audit re-url-contracts4, 2026-10-21).** Every cRZURL method in 008128a0 and 008216c0..00823ef3:

| Method | Address | Behaviour |
|---|---|---|
| default constructor | 008216e0 | allowRelative false, parsed false, type 0, port 0, all strings empty. No parse. |
| construct from text | 00821c60 (body continues at 00821c69) | full = the text; allowRelative **true**, parsed false, type 0, port 0, other strings empty. **No parse at construction**: the first lazy getter parses. |
| destructor | 008128a0 | releases the strings; no other effect. |
| assignment | 00821e20 | copies allowRelative, parsed, type, all nine strings (full, scheme, user, password, server, path, params, query, fragment) and port verbatim. No reparse. |
| GetFullURL | 008216c0 | returns full as stored. **Not lazy**, never parses. |
| GetPort | 008216d0 | returns the port field as stored. **Not lazy**: on a URL that no getter has parsed yet it returns 0 (or the stale value, see 2.7), even when full names a port. Call a lazy getter first if the port matters. |
| lazy getters | GetURLType 008236a0, GetScheme 00823710, GetUser 00823780, GetPassword 008237f0, GetServer 00823860, GetAbsolutePath 008238d0, GetParams 00823940, GetQuery 008239b0, GetFragment 00823a20 | parse `full` if parsed is false (2.3), then return the field. |
| component setters | scheme 00822000, user 00822030, password 00822060, server 00822090, path 008220c0, params 008220f0, query 00822120, fragment 00822150 | plain copy into the field. They change neither `parsed`, `type`, `port` nor `full`; `full` is only brought in line by MakeURLStringFromComponents (2.6). |
| SetFullURL (text, invalidate) | 00821fc0 | if invalidate is true, parsed = false; then full = text. Type, port and components are left as they were (a later lazy getter reparses only when parsed is false). The only caller (ParseCookieText 008f955f) passes invalidate = true. |
| IsURLRelative, IsURLRelative(text) | 008235f0, 00823660 | 2.2 |
| ParseScheme, ParseURL | 00822180, 008231b0 | 2.3, 2.4 |
| MakeURLStringFromComponents | 008233d0 | 2.6 |
| ChangeURL (new text, base text), ChangeURL (new text, base URL) | 00823a90, 00823ed0 | 2.7 |

There is no network, file or cache access in cRZURL and no error reporting beyond the boolean results of ParseURL and
ChangeURL; no method allocates beyond its strings.

### 2.2 Type codes

| Code | Scheme | Parse route after the scheme step (ParseURL 008231b0) |
|---|---|---|
| 0 | not recognised | none; ParseURL returns false |
| 1 | any other syntactically valid `scheme:` (allowRelative only) | path = rest |
| 2 | http (also https, see quirk) | user-info, server/port, hierarchical path |
| 3 | https | same as 2 (unreachable from a scheme, see quirk) |
| 4 | file | server step (strips `//` only), file path |
| 5 | ftp | user-info, server/port, ftp path |
| 6 | mailto | opaque path |
| 7 | news, nntp | opaque path |
| 8 | irc | user-info, server/port, hierarchical path |
| 9 | relative (allowRelative, no scheme) | server/port, hierarchical path |

IsURLRelative is `type == 9`. The second IsURLRelative overload (00823660) builds a temporary URL from a string with the
allowRelative constructor and asks it.

### 2.3 Scheme step (ParseScheme 00822180)

Work on a copy of the input, called *rest*. Clear scheme, type = 0.

1. If rest starts with `URL:` or `url:` (exact case, these two spellings only), erase those 4 characters.
2. If rest starts with `mailto:`: scheme = `mailto`, erase 7 characters, **stop with type 0** *[quirk]*: the
   branch at 00822246..00822256 jumps to the erase-and-return tail without storing a type, so ParseURL then fails and
   `mailto:` URLs never reach type 6 through this path.
3. Find the first `://`. If found at index i: scheme = rest[0,i) converted to lower case character by character;
   erase rest[0, i+1) (the `:`), so rest keeps the leading `//`.
4. If scheme is not empty, classify it by prefix (exact, on the lowered text), first match wins: starts with `http`
   -> 2; starts with `https` -> 3; starts with `file` -> 4; starts with `ftp` -> 5; else contains `mailto` at index 0
   -> 6; `news` at 0 -> 7; `nntp` at 0 -> 7; `irc` at 0 -> 8. *[quirk]* Because `http` is tested first and is a prefix
   of `https`, an `https://` URL gets type **2**; type 3 is never produced. Any scheme starting with `http` (for
   example `httpx`) is also 2.
5. If a type was set, stop (success). A `://` scheme that matches none of the prefixes (for example `gopher`) is
   **kept** in the scheme field: the scheme field is cleared only once, at the start of this step (00822187..0082219b),
   and steps 6/7 never touch it except to overwrite it in step 7's type-1 case. So `gopher://h/x` with allowRelative
   true ends as type 9, scheme `gopher`, server `h`, path `/x` (rest is `//h/x`, which has no `:`, so step 7 keeps type
   9); ChangeURL (2.7) then treats it as "N has a scheme". In guess mode the same input is type 0 (rest `//h/x` matches
   no guess prefix; `//` is inserted in front again) and ParseURL fails.
6. allowRelative false (guess mode): look at the start of rest — `www.` -> 2, `ftp.` -> 5, `news.` -> 7, `nntp.` -> 7,
   `irc.` -> 8, otherwise type stays 0. In every guess-mode outcome (also type 0) insert `//` at the front of rest, so
   the host becomes the server in the next step. Stop.
7. allowRelative true: type = 9. Find the first `:` in rest at index j. If j exists, j > 0, and every character of
   rest[0,j) is a letter, digit, `+`, `.` or `-` (C `isalnum` in the C locale or one of those three), then type = 1,
   scheme = rest[0,j) (case kept), erase rest[0, j+1). Otherwise type stays 9.

### 2.4 Component steps

Run in the order of table 2.2 on *rest*; each consumes what it reads. After the last step rest is cleared.

**User-info (008226d0)** — clears user and password. Only if rest is at least 2 long and starts with `//`: find the
first `@`; if found at k: user = rest[0,k) **including the leading `//`** *[quirk]* (checked at 0082273a..0082277e: the
copy starts at the string start), erase rest[0, k+1); then if user contains `:` at c, password = user after c and user
is cut to user[0,c). Consequence: since the `//` went into user, the server step that follows finds no `//`, so for
`http://u:p@host/x` the result is user `//u`, password `p`, server empty, path `host/x`.

**Server and port (008227e0)** — clears server, port = 0. Only if rest starts with `//`: erase the 2 characters. Then,
unless type is 4 (file, which stops here with server empty), let n = length, c = first `:`, s = first `/`:
- If c exists and c < s (or no `/`): the port run is the maximal run of ASCII decimal digits `0`-`9` right after c
  (the classifier at 008228b9 is CRT `isdigit`: 009ef61d tests ctype mask 4, `_DIGIT`, C locale). The run is
  counted in full, but only its first 15 characters are copied into a 16-byte buffer (008228c5: copy only while the
  count is below 15). If the run is non-empty, port = `atol` of the buffer. server = rest[0,c); then rest[0, c + 1 +
  **whole run length**) is erased (00822914), so no digit of the run is left in rest even past the 15th.
  `atol` (009ef7c2) accumulates value = value * 10 + digit in 32 bits with **no overflow check**: the result wraps
  modulo 2^32 and is stored as the unsigned 32-bit port (`:4294967296` gives port 0, `:4294967297` gives 1,
  `:99999` gives 99999; no range check against 65535 anywhere in cRZURL). A run of 16 or more digits is undefined in
  the original (buffer byte 15 is left unwritten and the terminator lands past the buffer); implementers use the
  first 15 digits, which is what the original yields whenever that stale byte is not a digit.
- Else if s exists: server = rest[0,s); erase rest[0,s) (the `/` stays in rest).
- Else (no `/`): for type 2 or 3 only, cut at the first `;`, then (if server still empty) the first `?`, then (if still
  empty) the first `#`: server = rest before it, rest keeps the delimiter onward. If server is still empty after that,
  server = all of rest and rest is emptied.

**Hierarchical path (00822a10)** — clears path, params, query, fragment.
- rest empty: path = `/` unless type is 9 (a relative URL keeps an empty path). Done.
- `;` first (before any `?` and `#`): end = index of `?` if present, else of `#` if present, else n. path = rest
  before `;`; params = rest[`;`, end); erase to end.
- Then if a `?` comes before any `#`: end = `#` or n; path (if still empty) = rest before `?`; query = rest[`?`, end);
  erase to end.
- Then if a `#` exists: path (if still empty) = rest before it; fragment = rest from `#` to the end; rest emptied.
- Finally, if path is still empty, path = rest.

**File path (00822d50)** — if server is exactly `localhost` (9 bytes, case-sensitive) server is cleared (never true in
practice, since the file server step leaves server empty). Clears params, query, fragment; path = rest.

**FTP path (00822df0)** — clears path, params, query, fragment. rest empty -> path = `/`. Else if rest has a `;` at k:
path = rest[0,k), params = rest[k, end). Else path = rest.

**Opaque path (00822f30, types 6 and 7)** — clears path..fragment, path = rest; succeeds only if path is non-empty.

**Other scheme (00822fc0, type 1)** — path = rest (params etc. not cleared); succeeds.

ParseURL (008231b0) clears every component and the port first, runs the scheme step, then the route, and stores the
route's result in `parsed`. Types 2, 3, 4, 5, 8, 9 always succeed once routed; 6 and 7 succeed when the path is
non-empty; 0 fails.

### 2.5 Worked examples (allowRelative true unless noted)

| Input | type | scheme | server | port | path | params / query / fragment |
|---|---|---|---|---|---|---|
| `http://www.simcity.com:8080/a/b.html?x=1#top` | 2 | http | www.simcity.com | 8080 | /a/b.html | - / `?x=1` / `#top` |
| `HTTPS://Host/` | 2 | https | Host | 0 | / | - |
| `http://host` | 2 | http | host | 0 | / | - |
| `http://host?q` | 2 | http | host | 0 | (empty: the `/` default applies only when rest is empty on entry) | - / `?q` / - |
| `ftp://host/pub/f.txt;type=i` | 5 | ftp | host | 0 | /pub/f.txt | `;type=i` |
| `file:///C:/x.htm` | 4 | file | (empty) | 0 | /C:/x.htm | - |
| `images/a.png` | 9 | (empty) | (empty) | 0 | images/a.png | - |
| `about:blank` | 1 | about | (empty) | 0 | blank | - |
| `www.maxis.com/x` (default constructor, guess mode) | 2 | (empty) | www.maxis.com | 0 | /x | - |

### 2.6 Rebuild (MakeURLStringFromComponents 008233d0)

full = scheme + `:`; then `//` unless type is 6; then, if user or password is non-empty: user, then `:` + password if
password is non-empty, then `@`; then server; then `:%u` of port if port is non-zero; then path, params, query,
fragment, each verbatim.

### 2.7 Resolve (ChangeURL 00823a90, arguments: new text, base text)

Clears parsed, type, port and full/scheme/password/server/path.
- Base empty: full = new; parse it (2.3/2.4) and return the parse result.
- Base non-empty and new empty: full = base (unparsed), return true.
- Otherwise build N from new and B from base, both with the allowRelative constructor, and parse both.
  - N has a scheme: full = new (unparsed); return true.
  - N has no scheme: N inherits from B each of scheme, server, user, password and port (port when N's is 0) that is
    empty in N. Then the path:
    - N path empty: N path = B's path. Then a chain, each test on **N's own field** and each step only reached when
      the previous test found it empty (00823d6c..00823deb):
      1. if N's params are non-empty, stop; else N params = B params;
      2. if N's query is non-empty, stop; else N query = B query;
      3. if N's fragment is non-empty, stop; else N fragment = B fragment.
      A non-empty N field is never overwritten. Examples (base `http://a/b;p?old#f`): new `?new` -> params `;p`,
      query `?new`, fragment empty (stops at 2) -> `http://a/b;p?new`; new `#g` -> `http://a/b;p?old#g`; new `;q` ->
      `http://a/b;q` (stops at 1: B's query and fragment are not inherited).
    - N path not starting with `/`: start from B's path, cut after its last `/` (keep the `/`; if B's path has no `/`
      keep it whole), append N's path, normalise (2.8), and use that.
    - N path starting with `/`: kept.
    Rebuild N's text (2.6), then copy N into this object (full, components, parsed, allowRelative). Return true.

Details (re-url-contracts4, 00823a90 and 00823ed0):
- The clear at entry covers parsed, type, port, full, scheme, password, server and path only. **user, params, query
  and fragment keep their old values** on the two early-return paths below; they are harmless for lazy getters (a
  reparse clears every component first, 2.4) but GetPort returns 0 until some lazy getter has run.
- A null base text is treated as empty. "Base empty" parses with **this object's own allowRelative** (not forced
  true) and returns that parse result; nothing else is reset.
- "New empty, base non-empty" and "N has a scheme": full is set (to base, resp. to new) and the call returns true
  with parsed false and type 0; the next lazy getter parses full with this object's own allowRelative, which is left
  unchanged on these paths.
- The parse results of N and B are not checked. "N has a scheme" is GetScheme on N being non-empty, so it includes
  type-1 schemes (`about:`), unrecognised `xyz://` schemes (2.3 step 5) and `mailto:` (type 0, scheme `mailto`).
- Inheritance: B's scheme is copied whenever N's scheme is empty (always true on this branch); server, user and
  password each only when N's is empty; port only when N's port is 0.
- On the full-resolve path this object ends with allowRelative **true** and parsed as N's (true when N parsed; N's
  type kept, so a relative N stays type 9 even after inheriting `http` from B).
- Overload 00823ed0 (new text, base URL object): if the base object is null or its stored full text (GetFullURL, not
  rebuilt, not parsed) is empty, it calls 00823a90 with a null base; otherwise with the base's full text. Its only
  caller is cGZWinHTML::SetURL (008fb25c).

### 2.8 Path normalisation (00821890)

Applied to the joined path in 2.7 only, in this order:
1. Repeatedly find `./` starting the search at 0 and then one past the last hit; erase the two characters unless the
   hit is preceded by `.` (so `../` is left alone).
2. If the path ends in `.` and is exactly `.` or ends in `/.`, erase that final `.`.
3. If the length is over 3: repeatedly find the first `/../`. At index 0 erase the first 3 characters (`/../x` ->
   `/x`). Otherwise find the `/` before it and erase from after that `/` through the `/../` (`/a/b/../c` -> `/a/c`);
   with no earlier `/`, erase from the start.
4. If the length is over 3 and the path ends in `/..` (its last `/` is 3 from the end): erase the segment before it
   and the `..` (`/a/b/..` -> `/a/`).

Example: base `http://h/docs/en/page.html`, new `../img/./x.png` -> joined `/docs/en/../img/./x.png` -> step 1
`/docs/en/../img/x.png` -> step 3 `/docs/img/x.png`; result `http://h/docs/img/x.png`.

## 3. cRZCookieManager — cookie store (RFC 2109 style)

### 3.1 Cookie record (CookieData, 0x94 bytes)

| Field | Default after parse | Set by attribute (name compared case-insensitively) |
|---|---|---|
| text | the Set-Cookie text as given | parse appends `;  domain=<d>` (two spaces) and `; path=<p>` when it derives them (3.2) |
| pairs | list of (name, value) in text order | every `;`-separated item |
| comment, commentURL | empty | `Comment`, `CommentURL` |
| domain | empty | `Domain`; a value not starting with `.` gets a `.` inserted in front |
| path | empty | `Path` (verbatim) |
| version | 0 | `Version` = `atol(value)` |
| expires (unsigned seconds since 1970) | **2 = session cookie** | `Discard` -> 1; `Expires` -> the HTTP date parsed by cRZHTTPClient::TimeStringToSeconds, or now + 86400 when that yields 0; `Max-Age` -> now + `atol(value)` (later attributes overwrite earlier ones) |
| ports (up to 16, 0-terminated) | first entry 0 = any port | `Port` (3.2) |
| secure | false | `Secure` -> true |

### 3.2 Parse (ParseCookieText 008f94f9; arguments: cookie, optional request URL)

Under the manager lock; now = `time(NULL)`. Reset every field to the defaults above. Split the text at `;` into at
most 40 items with surrounding blanks trimmed; each item splits at its first `=` into name and value (no `=`: the item
is the name, the value is empty). Then walk the pairs and apply the attribute column above. `Port`: split the value at
`,` (at most 16); with no items the list becomes { request URL port, or 80 when there is no URL or its port is 0 };
with n items the first **n-1** are stored as `atol` values and entry n-1 is set to 0.
This really drops the final item: the splitter returns an item count, not count plus one
(`00819448`, `008195b2–008195c0`; consumer `008f94f9`). A single explicit port
therefore produces an unrestricted list, whereas an empty Port value selects the URL port/80.

The splitter (`00819290`) trims only space and tab, not CR/LF or other ASCII whitespace.
For nonempty input, each delimiter completes an item even if empty; the final item is counted
only if it has bytes **before** trimming. Thus `a;;` has two items, `a; ` has two, empty
input has zero, and all-space input has one empty item. Stop at the capacity (40 for cookies,
16 for ports). Cookie splitting enables double-quote grouping: quote bytes toggle grouping
and are consumed; semicolons inside quotes remain data. Port splitting disables grouping,
so quotes are ordinary bytes. After whole-item trimming, splitting at the first equals sign
does not separately trim name/value: `a = b` has name `a ` and value ` b`.
The decimal conversions use the shipped 32-bit `atol`: locale-classified leading whitespace,
optional sign, digits until the first nondigit, zero without digits; accumulation and final
negation wrap modulo 2^32, with no overflow error or saturation (`009ef7c2`, arithmetic
`009ef834–009ef847`).

Derived defaults, only when the attribute is absent:
- domain: with a URL, take its server, then drop characters from the front one at a time until the text starts with
  `.` or is empty (`www.simcity.com` -> `.simcity.com`, `localhost` -> empty). If the result is non-empty append
  `;  domain=` + domain to the text.
- path: with a URL, the URL's whole path (not cut to its directory: `/a/b.html` stays `/a/b.html`); without a URL,
  `/`. If non-empty append `; path=` + path to the text.

### 3.3 Add (AddCookie 008fa054; arguments: cookie, optional request URL)

1. A cookie with no pairs: if its text is empty fail; else parse a copy (3.2, same URL) and add that copy instead.
2. With a URL, reject (return false) unless all hold:
   - the cookie path is non-empty and the URL path starts with it;
   - the domain is longer than 2, does not end in `.`, contains a `.`, and if it starts with `.` has another `.`
     after index 1;
   - the URL server ends with the domain (the first occurrence of the domain in the server must be the suffix;
     case-sensitive);
   - the part of the server before the domain contains no `.` (RFC 2109 one-level rule);
   - the port list is empty (first entry 0) or contains the URL port (80 when the URL has none), scanning at most 16
     entries and stopping at a 0 (008f6813).
3. Store: remove an existing record with the same first pair name, the same domain and the same path (exact byte
   compare, both must have pairs; 008f6d93). Then, if the cookie is alive (now < expires, or expires = 2): when the
   store holds at least its capacity, evict (3.5); append the cookie and return true. A cookie that is not alive is
   not stored and the call returns false; that is how a server deletes a cookie (the old copy was removed above).

### 3.4 Lookup (GetCookieDataForURL 008f6e9a, GetCookieTextForURL 008f7507)

A stored cookie matches a URL when all hold, checked in store order:
- alive: now < expires or expires = 2. *[quirk]* `Discard` sets expires to 1, so a discard cookie never matches.
- secure is false, or the URL type is 3 (https). *[quirk]* https URLs parse as type 2 (2.3), so secure cookies never
  match.
- domain non-empty and the URL server ends with it (first-occurrence-is-suffix test as in 3.3);
- path non-empty and the URL path starts with it;
- port list empty, or the URL port (80 when 0) is among the first 16 entries (a miss skips the cookie *[inferred]*).

GetCookieDataForURL(url, out, k) returns the k-th match (0-based). The string-URL overloads (008f71ff, 008f7240) build
an allowRelative URL from the string first; 008f8947 copies the match out with CookieData assignment.
On a copying-lookup miss it returns false and leaves the entire supplied output record unchanged
(`008f896d–008f8982`). A miss is not a reset-to-default operation.

Cookie header text (GetCookieTextForURL): clear the output; for k = 0..999 while a k-th match exists, for each of its
pairs in order:
- skip names `comment`, `commenturl`, `discard`, `expires`, `max-age`, `secure` (case-insensitive), and `version`
  once the version flag is set;
- unless this is the first pair of the first cookie, append `; `;
- names `domain`, `path`, `port`: append `$`;
- name `version`: if the output is non-empty, insert `$<name>=<value>; ` at the front, remove the last 2 characters
  (the `; ` just appended) and go to the next pair, *[quirk]* without setting the flag (008f76c5..008f76f1), so every
  later cookie's version is moved to the front again; if the output is empty, append `$` and set the flag;
- append `<name>=<value>` (stored spelling).

Return true when the output is non-empty. Example: stored `a=1; Version=1; Path=/` and `b=2; Path=/x`, both matching
-> `$Version=1; a=1; $Path=/; b=2; $Path=/x`.
The separator test uses original cookie and pair indices, not the number of pairs emitted.
Consequently a skipped first attribute can leave a leading `; `, e.g. a matching record with
pairs `Comment=x; a=1` emits `; a=1` (`008f7507`).

### 3.5 Eviction (008f9a92, called as evict(4, no domain filter))

First drop expired records (helper 008f9a07, returns the count removed *[inferred]*). While fewer than 4 were
removed and records remain, remove the record with the smallest expiry, counting a session cookie (2) as expiring
now *[inferred from the selection loop; the removal step was not re-read]*.

### 3.6 Persistence and settings (bounded persistence contract)

WriteCookieDataFile / ReadCookieDataFile store records as `CookieData%05u` entries under a `CookieData` key with
`Admin`/`Binary` mode marker. SetShouldEncryptCookieDataFile selects hexadecimal text storage;
despite the method name there is **no encryption**.

Capacity (Q2 answered 2026-10-22): the constructor reached from WWWSystem sets the manager's +0x10
capacity to **0x4000 = 16,384 records** (008f9bc5). Init does not assign that field: on its first call it
attempts the persistence read only when the record container is empty, then sets initialized and returns
true without testing that read's result (008f6a7c–008f6a99). The eviction trigger in section 3.3 therefore
uses this constructor default unless a separate caller changes the setting. This does not specify the
file loader's capacity handling or record format.

Further lifecycle and persistence facts (re-www-ww5, 2026-10-23), *[compat]*:

- Constructor `008f9bad–008f9bec`: initialized=false (+4), reference count=0 (+8), binary
  selection=false (+0xc), capacity=16384 (+0x10), critical section +0x14, empty cookie vector
  +0x1c..+0x24, and path +0x28 constructed from global pointer `00b16134`. That pointer's literal
  is `CookieData.ini` (PE pointer `00b16134` → `00acccb0`). Shutdown `008f9dda` writes then clears records and initialized only
  when initialized; returns true. EnumCookies `008f8bc7` copies the first min(requested count,
  stored count) records in store order, writes that count back and returns true under the lock.
- Writer `008f7784` takes the manager lock and calls ClearCookieDataFile first, including for
  an empty store. It writes only records whose expiration is **unsigned greater than now**:
  session sentinel 2 is therefore normally omitted. Surviving entries are numbered densely
  from `CookieData00000`, independent of their old indices (`008f7880–008f78a5`). Plain mode
  writes the stored cookie text through the string conversion, rather than serializing the
  in-memory 0x94-byte record. Binary mode writes an `Admin`/`Binary` marker, then passes text
  through helper `008f7752` to WriteBinaryEntry. The marker value is the string `1` (`00abb31c`).
  Helper `008f7752` is an identity copy through a temporary byte string (`00408a20`, `00408c40`),
  including for aliased input/output. WriteBinaryEntry `0091c722` encodes each stored text byte
  as two uppercase hexadecimal digits, high nibble first (`0090fef4`), without a trailing NUL
  in the payload. There are no lengths, timestamps, pair counts or record fields in that payload.
  For text `a=1`, the INI value is `613D31`. Plain-mode conversions `0090fbe5`/`0090f31b`
  copy length-bearing byte ranges; they do not transcode or interpret UTF-8.
- Reader `008f9e2b` clears all stored records before checking file existence. A missing file
  therefore leaves an empty store and still returns true. Presence of an `Admin`/`Binary`
  entry selects binary reads; the read's success boolean selects the mode, not a parsed
  boolean value of its text. Otherwise use ordinary string reads. Read sequential
  `CookieData%05u` names, indices 0..9999 inclusive, stopping at the first failed string read
  or zero-length binary read. Binary reads use a 16384-byte scratch buffer and the same
  helper `008f7752`. Each obtained text is admitted through AddCookie(text, null URL), with
  its return value ignored (`008f9ef8–008f9f86`). Read returns true after cleanup, not a count
  of successfully accepted records. These are text re-admissions: do not restore raw pointers,
  locks or absolute record layouts from disk. Binary reads decode pairs of hex digits, accepting
  upper/lowercase A–F (`0090ff33`), up to the first 32768 encoded characters into the 16384-byte
  buffer; invalid digits cause a zero read result and end the sequence. Odd-length encoded
  input is outside the safe paired-input domain (the converter consumes two characters per byte).

SetShouldEncryptCookieDataFile is a plain byte assignment, without rewriting the current file
(`008f6809`). SetCookieDataFilePath is **not** merely an assignment: under the lock, if the
new length-bearing path differs byte-for-byte, assign it then call ReadCookieDataFile (slot 18),
discarding that call's result; equal paths do nothing. It returns true either way and performs
no path normalization (`008f80aa–008f80ef`). A changed path to a missing file clears the store.

### 3.7 Cookie-manager dispatch inventory and distinguishing vectors

Table `00accf68`, zero-based slots, address facts from the export:

| Slots | Methods (entries in the same order) |
|---|---|
| 0–4 | destructor `008fa2fb`; Init `008f6a7c`; Shutdown `008f9dda`; AddRef `008f67a7`; Release `008f67ce` |
| 5–7 | SetShouldEncryptCookieDataFile `008f6809`; GetCookieDataFilePath `008f807b`; SetCookieDataFilePath `008f80aa` |
| 8–10 | AddCookie record `008fa054`; AddCookie text `008f8b5a`; RemoveAllCookies `008f9dfd` |
| 11–14 | GetCookieTextForURL parsed URL `008f7507`; string URL `008f71ff`; EnumCookies `008f8bc7`; ParseCookieText `008f94f9` |
| 15–19 | GetCookieDataForURL parsed URL `008f8947`; string URL `008f7240`; ClearCookieDataFile `008f683f`; ReadCookieDataFile `008f9e2b`; WriteCookieDataFile `008f7784` |

The cookie store has no drawing or socket callback interface; it synchronously supplies text/data
to WWW/HTTP and persists through INI IO. Proposed/unexecuted vectors:

| ID | Fixture | Required outcome |
|---|---|---|
| CK01 | Existing records; ReadCookieDataFile on nonexistent path | true, records empty |
| CK02 | Writer at now=100; expiries 2, 100, 101 | only expiry 101 is emitted, as CookieData00000 |
| CK03 | Reader entries 00000 present, 00001 absent, 00002 present | attempts admission only for 00000; stops at gap |
| CK04 | Three records; enumeration request count=2 | first two copied; count=2, true |
| CK05 | Three records; enumeration request count=5 | first three copied; count=3, true |
| CK06 | Empty store; WriteCookieDataFile | clear-file operation still occurs; true |
| CK07 | Parse `a=1; Port=80,81,82` | ports 80,81,0; port 82 dropped |
| CK08 | Parse `a=1; Port=81` / `a=1; Port=` without URL | unrestricted ports / 80,0 |
| CK09 | Parse ` a = b\t;\rPath=/x; ` | first name `a `, value ` b`; CR-prefixed name is not Path; final empty pair retained |
| CK10 | Copying lookup miss with prefilled output | false; all output fields preserved |
| CK11 | Binary writer with live text `a=1` | Admin/Binary=`1`; CookieData/CookieData00000=`613D31` |
| CK12 | Change path to missing file / assign equal path | first clears records through read; second preserves them |
| CK13 | `atol` values `4294967297`, `2147483648`, `-4294967297` | 1, signed -2147483648, -1 respectively |

## 4. cGZWWWSystem — front end and owned helper boundary

Evidence: native export and class/unnamed pseudocode, with instruction checks at the addresses below.
The coverage snapshot has **148 bodies / 34,049 B** for this owner, including reached and neighbouring
FTP implementation bodies. This is an attribution inventory, not proof that all those bodies implement
the public WWW interface. No cRZHTTPClient, NetworkFileCache, cookie-manager or socket-owner credit follows.
Foreign file, clock, thread and HTTP operations below are supplied inputs/calls; their success is never
promised by WWWSystem. Section 4 is a partial-owner contract until every body in its checklist is resolved.

### 4.1 Interfaces, identity and lifecycle

The constructor registers base service identity **0x449b1939**, with the second base-constructor argument zero
(008f9c02). It builds two empty job lists and a URL-handler list. The next-job counter initially holds 1;
allocation increments before use, so the first allocated WWW job ID is 2 (008f9c38, 008f83ee).
Cookie use starts enabled and cache use disabled (008f9c69–008f9c6d). Default custom data starts zero.
The constructor creates a file cache, cookie manager, HTTP client and FTP client, retains each through its
own interface convention and connects the two clients back to this service. Allocation failure is not
a clean false-return path: constructor virtual calls follow even a null allocation result.

`QueryInterface` accepts these IDs; successful queries retain the same object exactly once. An unsupported
ID returns false and leaves the caller's output untouched (008f687d–008f68c3):

| ID | Returned view relative to complete-object identity |
|---|---|
| 1 | public WWW view, +4 |
| 0x287fb697 | system-service view, +8 |
| 0x449b1912 | public WWW view, +4 |
| 0x452294aa | message-target/complete-object view, +0 |

The public WWW vtable is 00acce48. Its QueryInterface/AddRef/Release thunks adjust back to the complete
object. The secondary views share reference ownership; they are not independent services.
`Init` sets the initialized flag if needed and always returns true. `PostInit` calls Init, cache Init,
cookie Init, HTTP-client initialization and the FTP initialization helper, in that order, and returns true
without testing their returns (008f68fe–008f6929). The HTTP client's +0x10 target is 008179f0, which sets
its initialized byte at +4; the FTP helper 0045e110 sets its initialized byte at +8. Symbol names for
these folded/mismatched bodies are misleading; neither call releases the client.

`Shutdown` is guarded by the initialized flag. When set, it calls CancelAllDocumentLoadRequests,
RemoveAllURLHandlers, disconnects HTTP then FTP from the WWW service, calls cache Shutdown then cookie
Shutdown, and clears the flag. It returns true whether or not the guard was set (008f692a–008f696a).
The destructor invokes Shutdown before releasing its retained components and default target and destroying
containers (008f8992). Shutdown is not evidence of a successful underlying network transfer.

### 4.2 Load dispatch, target references and synchronous completion

Both LoadDocument overloads return an integer job ID, with zero used for rejection or failure. The string
overload constructs a URL and delegates to the URL-object overload (008f72b5). That overload selects the
explicit target, otherwise the stored default target. With neither, it returns zero before dispatch.
It retains the selected target once. A zero custom-data argument selects the default custom data;
zero cannot explicitly override a nonzero default. Preference 4 invokes the URL-path preference classifier
before dispatch (008f834d–008f837b).

Registered URL handlers are tested in list order, before built-in URL types. The first handler whose
acceptance call returns true owns the attempt: allocate a WWW ID, form a type-0 job, call that handler's
load method, and synchronously notify the target of success or failure. A false handler load does not
try another handler or the built-in protocol. The selected target reference is released after notification,
then the temporary job is destroyed; the allocated ID is returned even if the handler reports failure
(008f838a–008f848e). These jobs are never inserted in either asynchronous job list.

If no handler accepts, route URL types as follows (008f83a8–008f84e9):

| URL type | Attempt |
|---|---|
| 2 | HTTP helper 008f8ffe; pass the caller's cache-permission argument |
| 4 | local-file helper 008f84ea |
| 5 | FTP helper 008f93da |
| 7 | helper at 0090ce99 immediately returns 0; no callback and no target Release in that helper |
| any other | release the selected target reference and return zero; no notification |

URL parsing/resolution remains section 2's contract. WWWSystem consumes GetFullURL/GetAbsolutePath and
GetURLType; it does not implement a second URL parser or a separate HTTPS transport.

`SetDefaultNotificationTarget(new)` retains a nonnull new target **before** releasing the old target, then
stores it. Self-assignment therefore preserves lifetime. Null clears the setting (008f69e5). A dispatched
asynchronous job carries its own selected-target reference; changing the default does not retarget it.

### 4.3 Local files and representation preference

Local file loads are synchronous (008f84ea). With a target, allocate a WWW ID and construct a type-0 job,
copy the full URL, timestamp, preference, target and custom data. Derive the native path from the URL's
absolute path, apply helper 00823310, test existence, and if absent try the GetPathName-derived fallback.
The path conversion rules are given in section 4.11.

| Resolved preference | Existing-file behavior |
|---|---|
| 0 | Try opening and reading all bytes into RAM, independent of size. If opening fails, fall back to a filename. |
| 1 | Try opening; read into RAM only for unsigned size **< 500,000**. At exactly 500,000 or above, return a filename. Opening failure also falls back to a filename. |
| 2 or 3 | Return filename bytes without attempting a file read. |

The preference branch is a signed comparison against 1, while the size comparison is unsigned
(008f85d6, 008f8602–008f8607). An actual Read failure is a failure notification, not a filename fallback.
A filename result sets the job's file-data flag; a successful RAM read clears it. Success sets result=1
and progress=100; a missing path or failed read sets result=0 and progress=0. Notify even on this failure,
then release the target and destroy the temporary job. No completed-file history is retained.
The function returns zero for a missing path, but on the RAM-read branch it assigns the allocated ID before
testing Read success, so a failed Read can still return that nonzero ID (008f8624–008f8682).

### 4.4 HTTP admission, cached completion and duplicate URLs

Helper 008f8ffe requires a target and obtains the full URL and current timestamp. Under the WWW lock it
checks cache only when **both** service cache-use and per-request cache permission are true. A positive
IsCachedDataPresent result precedes duplicate-job lookup. Choose representation using the cache's unsigned
reported size and this table (008f9089–008f90dd):

| Preference | Attempt cached RAM data when |
|---|---|
| 0 | any size |
| 1 | size < 2,000,000 |
| 2 | size < 500,000 |
| 3 or higher | never |

If an eligible GetCachedDataAsData succeeds, copy the bytes and Release the returned CacheData. Otherwise
try GetCachedDataAsFile; success returns its filename and sets file-data. If neither retrieval succeeds,
continue to duplicate/network admission rather than notifying a cache failure. A usable cache hit allocates
a fresh WWW ID, forms a type-1 success/progress-100 job, unlocks, synchronously notifies and releases the
selected target reference. It does not insert a live job (008f912a–008f91a2).

On the non-cache path, scan active jobs for equal full-URL strings. A match allocates a fresh WWW ID and
appends a waiting job with its own preference, target and custom data, without submitting another HTTP
request (008f91aa–008f926a). The waiting job's foreign HTTP/FTP IDs start zero. Thus status lookup and
foreign-ID lookup need not agree that a waiting request exists.

With no duplicate, release the WWW lock, ask the cookie manager for request-header text if cookie use is
enabled, and, when supplied, submit one `Cookie` header. Enter the HTTP client's admission lock/helper,
submit method value 2 with URL and those headers, and only on a nonzero foreign ID allocate/store a WWW
job under the WWW lock. Release the HTTP admission helper after storing the job, then free temporary
header storage (008f9273–008f93d9). A foreign zero ID returns zero without releasing the selected target
reference acquired by the public overload. There is no WWW-level
retry loop in admission. Redirects, HTTP parsing, sockets and timeouts belong to the foreign client.

### 4.5 Live status and cancellation

GetDocumentLoadStatus locks and searches active then waiting lists; it returns **2** for a matching ID and
**0** otherwise. It reports live membership, not the job's result/progress field. Synchronously completed
jobs therefore immediately report 0 (008f6d08). GetJob searches **active only**, returns a borrowed pointer,
and adds no reference or lock (008f6d59). HTTPClientJobIDFromWWWSystemJobID returns the active job's foreign
ID only for type 1 or 2; FTPClientJobIDFromWWWSystemJobID only for type 3; absent/mismatched jobs return 0
(008f6a0c, 008f6a30).

CancelDocumentLoadRequest locks, searches active jobs, cancels the foreign HTTP request for type 1/2,
then erases that job. Type 3 is erased without a corresponding FTP cancellation call here. If no eligible
active match, search waiting jobs and erase a match. Return true on an erase, false on no match. It issues
no terminal notification (008f7e76). Erasing a Job destroys its URL/data containers, but its destructor
does not Release its target. Thus cancellation leaves the load's acquired target reference unreleased
(008f7451, 008f729b); do not infer an automatic COM smart pointer from the Job field.

CancelAllDocumentLoadRequests locks and collects a snapshot of IDs, active-list order followed by waiting-list
order. It calls the public single-request cancel method once per snapshot ID, then clears both remaining
lists. Its integer return is the snapshot count, **not** a boolean or count of successful cancellations
(008f7f04–008f7fb9). The export's misleadingly named helper 007ebd30 appends the pointed-to integer to a
vector; assembly 008f7f79–008f7f8a proves the per-ID cancel loop that the pseudocode removes as unreachable.

### 4.6 Notification envelope, callback lifetime and progress

All notification delivery here is a direct call to the selected target's message method (+0x0c), not a
queued message-server post. Terminal helper 008f6a9a constructs a standard message, retains it locally,
sets these fields, calls the target, then releases the message:

| Field | Terminal notification | Progress notification (008f6b15) |
|---|---|---|
| Message type | 0x75758483 | 0x75758484 |
| Word at message +0x10 (virtual setter +0x30) | public WWW interface pointer | public WWW interface pointer |
| Word at message +0x14 (virtual setters +0x40/+0x44) | boolean: Job result at +0x1c is nonzero | pointer to three temporary progress words |
| Word at message +0x18 (virtual setter +0x58) | pointer to Job | pointer to Job |

The Job and progress words can be stack temporaries. Consumers must copy fields/data they need during
the callback; retaining the message does not extend the pointed-to Job or progress lifetime. The terminal
helper itself does not release the target: its caller does so after delivery. These are native message
slot roles, not a promise that the generic message stores owned objects (008f6ace–008f6b0c,
008f6b49–008f6b80).

DoMessage switches on the incoming message type (008f698b). Unknown types return true with no action.

| Incoming type | Handler |
|---|---|
| 0x789af322 | HTTP terminal, 008f8c5a |
| 0x789af323 | HTTP progress, 008f6bbc |
| 0xc4eccd8c | FTP terminal, 008f815f |
| 0xc4eccd8d | FTP progress, 008f6c47 |

Progress handlers obtain the foreign-job pointer through incoming getter +0x50, lock, scan the active list by
foreign ID, save its target and Job pointer, then unlock. If a target was found and incoming Data1 points
to progress data (getter +0x3c), copy exactly three words into a temporary and send the progress envelope. They do not
retain the target or update stored progress. Both return true. FTP progress checks a null foreign-job
pointer; HTTP progress dereferences it while scanning and therefore requires a valid pointer whenever an
active entry is examined. Waiting jobs receive no progress forwarding through these handlers.

### 4.7 HTTP completion and waiting-job fan-out

The HTTP terminal handler obtains foreign-job information and incoming success, then locks and finds an
active entry by its foreign ID. It copies the Job into a local temporary and removes the live entry before
unlocking. A missing foreign pointer or absent match causes no callback; it still returns true. A matched
job without a target similarly skips delivery. No source/type discriminator is tested during the ID scan
(008f8c5a).

Success requires both incoming success and a nonnull foreign resource pointer. Otherwise form result=0,
progress=0 and attach the foreign-job pointer. On success, preserve the underlying resource's bytes or
filename unless representation conversion succeeds:

- Preference 3 with RAM resource: request a new cache-file path, open it, write all resource bytes. Only
  after a successful Write mark the resource as file-data and replace its bytes with filename bytes. A
  conversion failure leaves RAM data and does not turn the transfer into a failure.
- Other preferences with file resource: open and inspect unsigned size. Read into RAM for preference 0
  at any size, preference 1 below 2,000,000, or preference 2 below 500,000. A failed Read restores filename
  bytes; successful Read clears file-data. Open failure/oversize leaves the filename. Equality at either
  threshold keeps file data (008f8dcc–008f8e91).

Set result=1/progress=100, copy the resource representation into the Job, attach the foreign-job pointer,
call the cache-insertion policy helper 008f7a50, then process response cookies through 008f70c3. These calls
precede the original target's terminal notification. Cache insertion failure is ignored for transfer success.
Cookie processing runs only when cookie-use is enabled: enumerate header 0x35 indices 0–31 until absent,
then header 0x34 indices 0–31 until absent, passing each to the cookie manager with the resource URL.
This bounds each header family independently, preserves their family/index order, and does not implement
cookie parsing inside WWWSystem.

Notify the original target, Release it, and clear the local target pointer. Only **after that callback**,
lock and scan waiting jobs for equal full URL; copy matching waiting jobs to a temporary list and erase
them from the waiting list. Unlock before fan-out. For each copied job, in list order, replace only the
completion Job's WWW ID, target and custom data, notify that target, Release it, and clear the local target
pointer (008f8f2a–008f8fd0). Waiting jobs inherit the original completion's representation/preference and
other fields, even if they requested different preferences; no second conversion or foreign request occurs.
Because selection happens after the original callback, reentrant cancellation can remove a waiting job
before fan-out. Removed waiting jobs report status 0 during their callbacks. The original ID already
reports status 0 in its own callback.

Finally, if a foreign resource was obtained, clear its file-data flag and empty its data bytes; the local
Job copy is destroyed. If no active match is found, control skips this resource cleanup as well as delivery
(008f8cfd, 008f8fea); it does not clear unrelated/late resources. The handler always returns true. WWWSystem does not implement an HTTP
retry/backoff policy here; late completion after cancellation finds no active target and is ignored for
delivery. The cache policy inside 008f7a50 is specified in §4.12.

### 4.8 FTP admission and completion boundary

FTP admission helper 008f93da gets a full URL and timestamp, calls foreign helper 00817600 with the URL
and two zero arguments, and on a nonzero foreign ID allocates/inserts a type-3 WWW Job under the WWW lock.
It stores preference, selected target, custom data and FTP-client pointer; it does not consult WWW cache or
duplicate-URL lists. On foreign admission failure it returns 0 without releasing the selected target
reference. The reached FTP scheduler and wire operations are separate unfinished owned bodies (Q7).

FTP terminal handler 008f815f matches and removes the active Job under lock before delivery. It requires
a valid incoming foreign-job pointer; there is no HTTP-style null guard. Incoming success zero yields
result=0/progress=0, empty bytes and file-data=false. Nonzero success yields result=1/progress=100 and the
foreign job's filename, initially file-data=true. For preferences other than 3 it attempts the same
file-to-RAM conversion thresholds as HTTP; failed conversion keeps filename and success. Deliver one
terminal notification, Release the target and destroy the local Job. Return true. It neither fans out
WWW waiting jobs nor caches data nor processes cookies in this body.

### 4.9 Settings, handler registration and visited query

Cookie/cache setters store their supplied boolean byte directly; getters return that byte. They do not
clear queued jobs or the cache. The default custom-data setter/getter just store/load their word.
GetCacheDirectory delegates to the cache's getter. IsVisitedURL, despite its name, delegates to the
cache's IsCachedDataPresent when the cache pointer exists, otherwise returns false; it does not consult a
separate browsing-history set and does not test the cache-use flag (008f6a4d–008f6a5f).

The preference classifier's name mentions MIME, but it actually extracts the URL's absolute-path extension,
lowercases it and chooses 1 for `.html`, `.htm`, `.shtml`, `.asp`, `.plg`; otherwise it chooses 3. It always
returns true, including when extension extraction fails (008f736e; literals 00accd40–00accd58).

AddURLHandler retains the argument before calling its initialization slot (+0x0c). On false it Releases
that retain and returns false; on true it appends the pointer and returns true. There is no duplicate or
null check in this body (008f708a). RemoveURLHandler removes the first equal pointer, calls its shutdown
slot (+0x10), then Releases it; absent pointers do nothing (008f6e2b). RemoveAllURLHandlers walks in list
order, calls shutdown and Release on each, then clears list storage (008f6cd6). Registration and removal
do not acquire the WWW job lock here. A handler owns any external resource work it performs.

SetCacheDirectory copies the supplied path, adds a trailing backslash when needed, sets the cookie data
filename to that directory plus `CookieData.ini`, updates HTTP directory and `_cache` setting via 00819650
and 008196b0, updates the FTP directory via 00813c50, then returns the cache SetCacheDirectory result.
Earlier side effects are not rolled back if the final cache call fails (008f7966). Empty paths stay empty;
both slash and backslash count as existing final separators (008f7980–008f799e). The reached HTTP/FTP
directory setters copy the string and append a backslash only when a nonempty path has neither final
separator; the HTTP `_cache` setter simply copies its argument (00819650, 00813c50, 008196b0).
No directory or network success is implied by this call sequence.

The body-by-body coverage boundary is [the WWW owner checklist](web-network-www-bodies.md). Unresolved
owned HTTP scheduler/resource helpers are Q9; these are not credited by the call-boundary descriptions above.

### 4.10 Independent contract vectors (constructed; not live-game results)

| ID | Inputs/fixture | Required observations |
|---|---|---|
| WWW-01 | Unsupported QueryInterface; output preset to sentinel | false; sentinel unchanged; no AddRef |
| WWW-02 | Set default target T twice, then clear | AddRef(T), AddRef(T), Release(T), Release(T), in that order |
| WWW-03 | Explicit target absent, default absent | load returns 0; no dispatch or notification |
| WWW-04 | First handler accepts but its load fails; second would succeed | first handler alone loads; failure notification before target Release; nonzero allocated ID; status 0 |
| WWW-05 | Existing file, preference 1, lengths 499,999 and 500,000 | first attempts RAM Read; second returns filename without Read |
| WWW-06 | File open succeeds but Read fails | failure/progress 0 notification; selected target released once; allocated ID may still be returned |
| WWW-07 | HTTP cache enabled/per-request allowed, preference 1; sizes 1,999,999 and 2,000,000 | first attempts RAM retrieval; second goes directly to file retrieval |
| WWW-08 | Cache-present true; RAM and file retrieval both fail; no active duplicate | proceed to foreign HTTP submission, no cache-failure notification |
| WWW-09 | Two loads of identical full URL while first is active | distinct WWW IDs, one foreign request; status 2 for both; GetJob/foreign-ID lookup finds active ID only |
| WWW-10 | Cancel active HTTP job, then same ID again | one foreign cancel and one erase, true then false; no callback |
| WWW-11 | Cancel active FTP job | erase and true; no FTP cancel call from this body |
| WWW-12 | Completed cache hit followed by GetDocumentLoadStatus | synchronous callback before load returns, then status 0 |
| WWW-13 | Active A, waiting B/C with same URL; A callback cancels B | A callback then C callback only; no lock held during callbacks; B cancellation leaks its acquired target reference |
| WWW-14 | A preference 3 completes with filename; waiting B requested 0 | B receives A's filename/representation and preference; no B-specific read |
| WWW-15 | Snapshot IDs A/B active then C waiting; cancel-all | cancel A, B, C in that order; return 3 even if a cancel fails; no terminal notifications |
| WWW-16 | Load type 7 with a target, or HTTP/FTP foreign admission returns 0 | return 0; no callback; acquired target reference has no balancing Release on these paths |
| WWW-17 | Two response header families, each with 33 cookies | indices 0–31 of family 0x35 first, then 0–31 of 0x34; index 32 ignored |
| WWW-18 | HTTP transfer succeeds but RAM-to-file Write fails | terminal success remains 1/progress 100 with RAM bytes; cache/cookie calls still precede notification |
| WWW-19 | Incoming HTTP completion after its WWW job was canceled | no target callback, return true; resource bytes/flag unchanged by this handler |
| WWW-20 | Cache-use false, cached URL present | IsVisitedURL may return true; HTTP load still bypasses cache retrieval |

### 4.11 Reached native-path conversion

Helper 00823310 copies its input. If its byte length is at least three, byte 0 is `/` and byte 2 is `|`,
replace the pipe with `:` then erase the initial slash. There is no drive-letter check. Next run decoder
00823000, then replace every `/` in the decoded bytes with `\`. Copy the result out and return true;
input/output aliasing is supported by the temporary copy (0082333f–0082339f).

Decoder 00823000 scans bytes left to right exactly once. A percent with two following bytes consumes all
three; two hexadecimal digits produce the low byte of their hexadecimal value, otherwise produce one
space. A percent with fewer than two following bytes is copied literally, as are all other bytes. It
neither changes plus to space nor recursively decodes newly produced percent bytes. NUL values are
stored as bytes in the length-bearing result; later foreign C-string consumers may interpret them
differently. Hex validation/conversion and the invalid-escape space are checked at 0082308b–008230ba.

| ID | Input to native-path helper | Output bytes |
|---|---|---|
| WWW-21 | `/C|/a%20b` | `C:\a b` |
| WWW-22 | `/1|/x` | `1:\x` (no alphabetic drive validation) |
| WWW-23 | `a%ZZb%2` | `a b%2` |
| WWW-24 | `a%252Fb+c%2F` | `a%2Fb+c\` (one decoding pass) |

### 4.12 HTTP response cache-insertion policy (008f7a50; Q8 answered)

This helper takes the completed foreign HTTP job and returns a byte boolean. A null job,
null request at job+0x14, or null response/resource at job+0x18 returns false without insertion.
It borrows these objects; it neither retains them nor calls a notification target. All temporary
header strings are destroyed before return. Its caller ignores false for transfer success (§4.7).

Read only occurrence zero of each named header, in this order: Pragma, Date, Last-Modified,
Expires, Cache-Control (header IDs 4, 3, 0x2d, 0x2c, 1; names verified at 008177b0).
Pragma containing the exact lowercase byte substring `no-cache` anywhere rejects insertion.
This is case-sensitive, without word boundaries: `xno-cachey` rejects; `NO-CACHE` does not.
The native test is unsigned find-position < full header length, including a match at position zero
(008f7ab0–008f7ac6; find wrapper 00442850 calls 004427f0).

Let D be Date parsed by TimeStringToSeconds (00818bd0), or the current 32-bit Unix seconds
only if Date is absent. An invalid but present Date is passed to that parser without a fallback.
Last-Modified, if present, is parsed, but its result is discarded. Let E be parsed Expires when
present; otherwise E is D + 259200 (three days). This addition is a single 32-bit wrapping ADD,
without saturation, freshness comparison or subtraction of current time (008f7af9–008f7b60).
Age is never read. An already-expired E is still passed to insertion.

Cache-Control processing deliberately preserves these native rules:

1. Split on commas outside double quotes, with six output slots. Double quotes toggle quoted
   mode and are omitted from token bytes; no backslash escape rule exists in this splitter.
   Trim ASCII space and tab from each token. Stop at six tokens, ignoring the remaining input.
   Delimiter-terminated empty tokens still consume slots; a final zero-length tail does not.
   A nonempty whitespace-only tail consumes a slot before trimming. This follows the called
   HTTPMessage splitter 00819290, not an HTTP directive grammar.
2. Compare each complete token using CRT case-insensitive string equality against `private`,
   `no-cache`, `must-revalidate`, `max-age`, and `no-store`. Copy recognized tokens into five
   separate local strings; repeated matches overwrite the corresponding string. There is no
   split at `=` before comparison (008f7be7–008f7c5e). The misleading export name at 00408c40
   is a string assignment, as its instructions confirm.
3. Reject if any copied private, no-cache or no-store string has nonzero byte length. The copied
   must-revalidate string is never tested. In particular `private=field`, `no-cache=field`,
   `no-store=1`, and `max-age=60` are unrecognized and cannot block or change expiry.
4. For a copied max-age string, find `=` and, only if its unsigned position is below length−1,
   erase the prefix through `=`, apply CRT atol to the remaining C string, and replace E with
   D + that 32-bit result, wrapping (008f7c95–008f7cc4). For ordinary header text this branch
   cannot change E: the only matching whole token is bare `max-age`, which contains no `=`.
   Do not repair this by parsing the argument of `max-age=...`. The branch description also
   specifies length-bearing strings with embedded NULs, for which C-string comparison and
   stored byte length need not agree.

Only after this policy work, check WWW cache-use and resource-data length. Disabled cache or
empty data returns false. For RAM data, call cache vslot +0x5c with request URL, data pointer,
byte length, flags (1,1), E. For filename data, convert filename bytes to an RZString and call
vslot +0x60 with request URL, filename, flags (0,1), E. Return that insertion call's low-byte
boolean; no retry, deletion or target callback occurs here (008f7d21–008f7e43).
These are borrowed call arguments: this helper does not transfer/release the job or resource.
Whether the cache internally copies or moves file contents belongs to its own interface contract.

**Independent distinguishing vectors (constructed; no network or game observations).** Use a
nonempty RAM response, enabled cache, Date parser result D=1000 and no Expires unless stated.
Observe cache-call count, E and flags through an interface test double, not a live server:

| Header input / variation | Expected cache boundary |
|---|---|
| None of the policy headers except Date | One +0x5c call, E=260200, flags (1,1) |
| Pragma `xno-cachey` | No insertion; false |
| Pragma `NO-CACHE` | One insertion with E=260200 |
| Cache-Control ` PRIVATE ` | No insertion; false |
| Cache-Control `private=field, no-store=1, max-age=60` | One insertion with E=260200 |
| Cache-Control `must-revalidate, max-age` | One insertion with E=260200 |
| Cache-Control `a,b,c,d,e,f,no-store` | One insertion: seventh token is ignored |
| Cache-Control `a,b,c,d,e,no-store,f` | No insertion: sixth token blocks |
| Cache-Control `"a,no-store", public` | One insertion: quoted comma stays within one token |
| Expires parses to 7; Last-Modified parses to 900 | One insertion with E=7 |
| Date parser result 0xffff0000, no Expires | E=0x0002f480 (32-bit wrap) |
| Same ordinary input with filename representation | One +0x60 call, flags (0,1), E=260200 |
| Disabled cache, empty bytes, or missing job/request/response | No insertion; false |
| Cache interface returns false | Helper false; enclosing HTTP completion still succeeds |

### 4.13 Reached FTP parser, transfers and helpers

See [the FTP native contracts](web-network-ftp.md) for 00812de0's complete selector table,
RETR/STOR entry/command/file/notification ordering, reached transport helpers, and independent
distinguishing vectors. Q7 remains open for login, listing acquisition, outer scheduling and
shutdown. The companion records unsafe native input domains explicitly and claims no live
network observations.

### 4.14 Reached HTTP admission and cancellation (Q9 partly resolved)

These are reached bodies attributed to WWWSystem by the fixed owner inventory, even though
they operate on the foreign HTTP client. They do not change WWW's own target-reference rules.
Names for still-untraced dependency functions are deliberately not inferred.

**Method conversion.** 00817d00 maps IDs 1–10 respectively to OPTIONS, GET, HEAD, POST, PUT,
DELETE, TRACE, PATCH, LINK, UNLINK; other IDs return null. 00817bd0 tests those exact uppercase
strings in that order using CRT strncmp and its caller-supplied unsigned length. It redundantly
tests PATCH twice, returns the matching ID, or zero for unknown. A short supplied length matches
a prefix (zero length matches OPTIONS); this is not an unconditional whole-word test. The
admission caller passes 0xffffffff, so ordinary terminated strings must match the whole method.
Literal bytes GET/POST at 00aa1368/00aa1378 and LINK/PUT at 00abfc6c/00abfc8c were checked in
the executable, rather than using the misleading pointer-symbol names in pseudocode.

**Submission wrapper 0081cda0 (319 B).** Arguments are method ID, URL string, optional body
string object, body representation byte, array of 24-byte records, record count, and custom
value. IDs 8, 9, 10 immediately return zero, despite the converters recognizing their names.
Otherwise allocate/construct a 0x78-byte request (0081a910), increment its reference count,
set its method string from 00817d00 and copy the URL. Only method 5 requires a nonnull body:
if absent, release the temporary request through 00817790 and return zero. For method 5,
copy the body's length-bearing string at body+4 into request+0x4c and set request+0x64 to
the supplied representation byte. Other methods ignore those two arguments in this wrapper.
Append the supplied 24-byte records in order, then call admission 0081ca90(request,custom).
Release the temporary request reference after that call, returning its ID/zero. Allocation
failure is not handled safely: a null request allocation is subsequently dereferenced.
Record count is decremented until zero, without validation of negative counts or array bounds.

**Admission 0081ca90 (266 B).** Reject with ID zero if request is null, method byte length is
zero (+0xc..+0x10), or URL byte length is zero (+0x20..+0x24). Before taking the queue lock:
resolve method with 00817bd0; when request+0x4c is nonempty and its record range +0x58..+0x5c
is empty, invoke 0081acf0 on that string and then erase the string. When the resolved method
is neither zero nor OPTIONS, the record range is nonempty, and its first record's byte
range +0xc..+0x10 is empty, call 0081ac20 on that first record. Those normalization helpers
are dependency boundaries, not newly specified parsers here (0081cac8–0081cb17).

Increment the request reference count at +4 **before** the lock. Form an eight-word job
record: [ID,1,0,1,0,request,0,custom]. Lock client critical section +0xa4, increment the
client's 32-bit counter at +0x3c, assign it as ID, append the record to the queue at +0xb0
with 00818100, and unlock. Then call worker activation 0081c9d0 and return the assigned ID
(0081cb1c–0081cb85). Counter wrap is not avoided: zero can be assigned/enqueued even though
the interface otherwise uses zero for failure. This body performs no notification itself.
The request is retained across successful queue insertion; URL and custom value are not
independently AddRef'd. The wrapper's release leaves the queue's added request reference.

**Queue append 00818100 (52 B).** Allocate a 0x28-byte node, copy exactly eight 32-bit words
from the supplied record into node+8, and link it immediately before the circular sentinel
(tail insertion). No field-specific copy, AddRef or cleanup is performed. The record is
copied; pointed-to objects remain shared. Allocation failure is not a recoverable return.

**Cancel 0081cc10 (191 B).** Hold +0xa4 throughout the following search/mutation, releasing
the lock before each return:

1. Scan queued nodes from head to sentinel by ID at node+8. On the first match, unlink and
   free the raw node, then return true. It does **not** Release the request in that record,
   notify, activate a worker, or continue searching for another equal ID.
2. Otherwise, if active-worker pointer +0xac is nonnull, its current-record pointer +0x6c is
   nonnull and that record's first word matches, set worker bytes +6 and +5 to 1, then clear
   client+0xac. If the queue has any nodes, call 0081c9d0 **while still locked**. Return true.
3. Otherwise return false, with no cancellation side effects (0081cc25–0081ccce).

Active cancellation does not join the old thread, close its socket, free its record, or
release its request here. Activation can allocate/start a successor while the old worker is
still alive. Consequently this cancellation return alone is not a guarantee of quiescence
or callback suppression. Exact worker handling of flags and final request/resource release
remains Q9; do not synthesize it from WWW's late-completion lookup behavior (§4.7).
The reached activation dependency 0081c9d0 was checked: it locks +0xa4, allocates worker
context when +0xac is null, clears its stop byte +5, and, if its running byte +4 is zero,
sets that byte, AddRefs the client and starts StaticWorkerThreadFunction. The static wrapper
0081c9b0 calls WorkerThreadFunction then Releases the client. No thread join was found in
these admission/cancel bodies; worker completion itself (0081ba80, another owner) was not
fully traced in this round. Nested lock acquisition is observable in the cancel path.

**Release helper 00817790 (22 B).** Interpret reference count at +4 as unsigned. If >1,
decrement it and return the new count; otherwise invoke deleting destructor vslot zero
with argument 1 and return zero. There is no special zero-count rejection or notification.

**Independent distinguishing vectors (constructed).** Use recorded dependency calls, controlled
thread activation and existing objects, not a live HTTP endpoint:

| Input / setup | Result |
|---|---|
| Method converter with `GETaway`, length 1 versus 0xffffffff | Method ID 2 versus unknown method 0; unknown is not itself an admission rejection |
| Zero comparison length with any valid input pointer | ID 1 (OPTIONS) |
| Wrapper IDs 8/9/10, otherwise valid arguments | Zero before request allocation |
| Wrapper PUT and absent body | Zero after releasing its temporary request; no queue entry |
| Null request, empty method or empty URL on admission | Zero with no queue entry or new request reference |
| Admission counter 0xffffffff | Enqueues ID 0 and returns 0; no wrap skip |
| Two queued records with the same ID | Cancel removes only the first, without releasing its request |
| Queued match and active record with same ID | Queue removal wins; active worker flags are untouched |
| Active match plus nonempty queue | Both worker flags set, active pointer cleared, activation called before unlock |
| Unknown cancellation ID | False after unlock, without notification |
| Request count 2 versus 1 on 00817790 | Return 1 without destruction versus deleting destructor and return 0 |

## 5. cRZHTTPClient — bounded client contract

HTTP/1.x client jobs, TimeStringToSeconds (HTTP dates), DoJobFinalization (resolves redirects with
cRZURL::ChangeURL), cookie send and receive through 3.3 and 3.4, disk cache through NetworkFileCache.

### 5.1 Inventory, construction and ownership boundaries

The constructor/lifetime, completion, cancellation and shutdown contracts are in
[HTTP lifetime](web-network-http-lifetime.md), sections 1–3. Submission, queue-record copying,
method mapping and request ownership are in section 4.14 above. These are the same reached
HTTP objects, not a second independent WWW implementation. The raw table `00abfca8` is
mislabelled as a particle simulator by the export: its six entries are slot 0 `00811cb0`,
1 `004bdf00` (folded AddRef), 2 `0048c920` (folded Release), 3 `0081d030`,
4 `008179f0` (Init association), 5 `0081cd40` (shutdown). Only the linked contract's
qualified effects may be inferred from these entries; names alone do not qualify slots 0/3.

Named nonvirtual inventory:

| Entry | Method / role | Qualified behavior |
|---|---|---|
| `00817d80` | TimeToSeconds | conversion boundary below |
| `00817df0` | WeekdayStringToInteger | date helper; full input contract remains Q13 |
| `00817ed0` | SendNotification | section 5.2 |
| `00818020` | DoJobUpdate | section 5.3 |
| `00818490`, `00818680`, `00818bd0` | GetDateTypeFromString, ParseDateTimeString, TimeStringToSeconds | date parsing residual Q13 |
| `008196e0` | DoJobCompletion | linked HTTP lifetime section 2 |
| `00819800` | ReadMessage | body framing/spill residual Q13 |
| `0081a470` | DoJobFinalization | residual Q13 |
| `0081ba80`, `0081c9b0` | WorkerThreadFunction, static wrapper | bounded stop/completion/exit ordering in linked HTTP lifetime |

Constructor defaults include timeout=15, suffix `.tmp`, null target and worker, empty request
queue and an allocated-or-null lookup helper; exact native offsets and remaining defaults are
in the linked lifetime section 1. It does not connect or start work. No drawing occurs here:
the HTML owner consumes resources/results through WWW. The HTTP client exposes IO and synchronous
notification payloads; drawing/layout belongs to HTML specs.

TimeToSeconds takes weekday/day/month/year/yearday/hour/minute/second, creates runtime calendar
fields with month/year offsets -1/-1900, weekday/yearday offsets -1, **tm_isdst=1**, and returns
32-bit mktime result minus the global word at `00b18ba8` (`00817d80–00817de5`). Do not replace
this with an unconditional UTC conversion: the runtime timezone and global adjustment are
dependencies, still Q13. TimeStringToSeconds returns zero on parser failure (`00818bd0`).

### 5.2 Notifications are synchronous borrowed-payload calls

*[compat]* SendNotification(type, object, payload-word-2, payload-word-3) does nothing when
the client's target +0x10 is null. Otherwise temporarily AddRef the non-null object, create and
retain a standard message, assign type through message slot +0x14, object through +0x30,
second payload through +0x44 and third through +0x58. Call the target's DoMessage at +0xc
synchronously, Release the message, then Release the object (`00817ed0–00817f57`). No message
server queue or target AddRef is introduced here. Allocation failure has no recoverable contract.
The last two payload words are not independently retained; a pointer to a local progress tuple
is valid only during delivery. Completion's resources remain retained until after delivery as
specified by the linked lifetime contract.

### 5.3 Progress calculation and threshold

DoJobUpdate(job, received, expected, previous-percent-pointer) returns false without delivery
unless target exists and **signed expected >2048**. Expected `0x7fffffff` means unknown length:
report expected=0 and percent=50. Otherwise percent is unsigned division of the low 32 bits
of received*100 by expected, capped at 100. Notify only if **unsigned (percent-previous) >=10**.
This subtraction wraps: a falling percent can pass the threshold. On delivery, send type
`0x789af323`, object=client, second payload=pointer to three 32-bit words
[received, normalized-expected, percent], third payload=job; then write percent to previous
and return true. A false return leaves previous untouched. Evidence: `00818020–008180b3`;
IMUL/DIV at `00818058–0081805d`, unsigned threshold at `00818075–0081807a`.

### 5.4 Independent vectors (PROPOSED / UNEXECUTED)

| ID | Input | Required outcome |
|---|---|---|
| HTTP01 | expected=2048, target present | false, no callback, previous unchanged |
| HTTP02 | received=1000, expected=10000, previous=0 | callback [1000,10000,10]; previous=10; true |
| HTTP03 | received=1900, expected=10000, previous=10 | percent=19, false; previous=10 |
| HTTP04 | received=2000, expected=10000, previous=10 | callback percent=20; previous=20 |
| HTTP05 | expected=2147483647, received=123, previous=0 | callback [123,0,50] |
| HTTP06 | received=1000, expected=10000, previous=90 | wrapped delta passes; callback percent=10 |
| HTTP07 | received=42949673, expected=10000, previous=0 | multiplication wraps to 4; percent=0; false |
| HTTP08 | target null; arbitrary valid notification arguments | no message allocation or temporary object hold |
| HTTP09 | notification with non-null object and target | object AddRef; message delivery; message Release; object Release in that order |

These vectors qualify helper behavior, not an entire HTTP transaction. Redirect limits,
wire request generation, informational replies, chunk extensions/trailers, connection-close
framing, body spill failures and finalization remain Q13. Do not silently substitute a browser's
HTTP policy or claim full-client conformance from progress/lifetime tests.

## 6. NetworkFileCache — storage and IO contract

Added 2026-10-23, re-www-ww5. *[compat]* applies to the contracts below. Evidence is the
Windows export, with numeric branches checked against instructions. There is no drawing interface:
this owner exchanges strings, byte buffers and file paths. WWW's admission/cache policy remains
in sections 4.6 and 4.12; the cache itself does not interpret HTTP headers. No live IO was run.

### 6.1 Dispatch inventory and construction

Native table `00acd418`, zero-based slots; each group below lists successive slots in order.
These are interface facts, not a claim that every helper's behavior is settled.

| Slots | Methods (entry addresses in the same order) |
|---|---|
| 0–4 | deleting destructor `008fd750`; Init `008fdde8`; Shutdown `008fd3d9`; AddRef `0094c4b1`; Release `008fc3ef` |
| 5–8 | GetCacheDirectory `008fcc18`; SetCacheDirectory `008fcc2a`; GetCacheIniFileName `008fcc87`; SetCacheIniFileName `008fcc99` |
| 9–11 | SetRAMCacheSize `00945edc`; SetFileCacheSize `00945ed0`; SetDefaultExpirationTime `0096c3cb` |
| 12–14 | IsCachedDataPresent `008fd0dc`; IsCachedDataPresentAsRAMData `008fd145`; IsCachedDataPresentAsFileData `008fd1b4` |
| 15–18 | GetCachedDataAsData `008fd3f6`; GetCachedDataAsFile `008fd5b0`; GetCachedFileInfo borrowed-pointer overload `008fc5b3`; copying overload `008fd227` |
| 19–22 | ForceCachedDataIntoFile `008fc613`; GetCachedDataSize `008fc405`; GetTimeCreated `008fc426`; GetTimeNow `008fc447` |
| 23–25 | AddNewCachedData bytes `008fd998`; AddNewCachedData source-file `008fd76c`; RemoveCachedData `008fd69c` |
| 26–27 | GetNewCacheFileName `008fcd49`; GetNewCacheFilePath `008fceeb` |

Nonvirtual support: RemoveCachedFile `008fc669`, RemoveUnusedCachedFiles `008fc6e0`,
WriteDataToFile `008fc7d7`, GetFileLength `008fc887`, UpdateCacheIniFile `008fc9b1`,
DoPeriodicCacheMaintenance `008fd258`, ClearCacheMap `008fd2cc`, IniFileCallbackFunction
`008fdb0d`, ReadCacheIniFile `008fdd39`; constructor `008fd389`.

Constructor defaults (`008fd38c–008fd3d5`): initialized=false (+4), reference count=0 (+8),
empty directory string (+0xc), INI-name string (+0x20) from global pointer `00b16144`,
empty map (+0x34), file-size setting=3,000,000 (+0x48), RAM-size setting=500,000 (+0x4c),
default lifetime=432,000 seconds (+0x50), two operation counters=0 (+0x54/+0x58).
The INI name is `FileCache.ini` (PE `00b16144` → `00acd3d4`). Setters for the three numeric settings
store their entire 32-bit argument and return true; they perform no immediate trim, validation
or IO (`00945ed0`, `00945edc`, `0096c3cb`). No capacity-enforcement promise follows from their names.

### 6.2 Lifecycle and path settings

Init is idempotent. On its first call, an empty directory is replaced by the temporary directory
plus `Maxis Network Cache\`. It checks directory existence and attempts creation if absent,
reads the INI, removes unused cache files, sets initialized and returns true. It ignores creation
and read failures (`008fdde8–008fde3e`). Shutdown, only when initialized, writes the INI,
clears the map and clears initialized; it returns true (`008fd3d9`). Thus a true lifecycle result
does not certify a successful disk operation. No thread, callback or asynchronous message is
created by these entries.

SetCacheDirectory assigns before validation: empty returns false, leaving the empty assignment.
A nonempty path gets a trailing backslash unless its last byte is already a path separator;
an existing directory succeeds, otherwise return the directory-creation result. Failure does
not restore the old directory (`008fcc2a–008fcc86`). SetCacheIniFileName first checks/removes the
old directory+INI-name file, assigns the new name and returns true (`008fcc99`). It does not
rename the old file or migrate cached records. Directory and INI getters return their stored strings.

### 6.3 Entries, expiration and reads

An entry has a key, relative filename string, optional reference-counted RAM data, byte length,
an unsigned byte-derived attribute, lifetime, creation time, last-access time and expiration time.
Relative to the info record these are filename +0, RAM holder +0x14, length +0x18,
attribute +0x1c, lifetime +0x20, creation +0x24, access +0x28, expiration +0x2c.
Offsets are interoperability facts for the borrowed/copy info methods, not a portable layout mandate.
RAM holders contain buffer pointer +0, size +4, 16-bit reference count +8.

Presence/read methods look up the key. A missing entry returns false. A found entry is live only
when **signed 32-bit now < signed expiration**; equality is expired. On expiration they release
the map's RAM hold, attempt to remove its file and erase the entry, returning false
(`008fd10a–008fd133`, `008fd3f6`, `008fd5ef–008fd61b`). PresenceAsRAM additionally requires a
non-null holder; PresenceAsFile requires a nonempty stored filename. Neither tests disk existence.
Info getters do not perform this expiration test: borrowed lookup returns the entry address on
success; copying lookup assigns its contents. Size/time getters use the info lookup and return
zero on absence (`008fc405`, `008fc426`, `008fc5b3`, `008fd227`).

GetCachedDataAsData(key, optional output-holder-pointer) returns true for a live record with no
output requested, without loading its file. With an output requested, an existing RAM holder is
retained (16-bit count increment) and returned. Otherwise read directory+filename, obtaining length
from the file, allocate a holder and buffer and read that many bytes. Successful loading installs
the RAM holder in the entry and gives the caller a second hold. Open failure, zero length or read
failure removes the file and erases the entry; false leaves the caller's output untouched.
Successful calls update last-access to the sampled now (`008fd3f6–008fd5af`). Allocation failure
and oversized/refcount-overflow inputs have no qualified recoverable contract.

GetCachedDataAsFile(key, optional output-path) differs: materialization is attempted **only when
an output path is supplied**, the record has RAM and its filename is empty. Generate/reserve a
filename and write the RAM bytes. A write failure clears that filename. The output is always
assigned directory+the resulting filename for a live record; failure can therefore leave the
directory alone in output. Update access time even if materialization failed, then return whether
the filename is nonempty. With null output, do not generate or write: report existing filename
state and update access (`008fd629–008fd698`). ForceCachedDataIntoFile uses a temporary output
string and returns this same result (`008fc613`).

### 6.4 Adding bytes, removal and maintenance

AddNewCachedData(key, bytes, size, copy, attribute-byte, lifetime) rejects an empty key before
changing counters or old entries. For a nonempty key, increment both counters and run periodic
maintenance when the second is **unsigned >40**. Remove any old same-key record and file **before**
testing bytes/size. Null bytes or zero size then returns false: this is destructive replacement,
not a no-op (`008fd998–008fda27`).

For non-null, nonempty data, create a holder. With copy=true allocate/copy the bytes; otherwise
store the passed buffer pointer. Do not treat the latter as a const borrowed slice with automatic
  caller ownership: native holder destruction/allocator compatibility must be supplied by the
  adapter. Attribute is zero-extended from one byte. Lifetime=-1 selects the manager default;
after substitution, any negative lifetime gives expiration `0x7fffffff`; otherwise expiration is
32-bit now+lifetime, with creation=access=now. No saturation is performed
(`008fda4b–008fda8d`). Insert the new entry and return true. Failed allocations are outside the
qualified domain, rather than a promised false return.

Data and file retrieval increment the same two counters before lookup, including misses.
Periodic maintenance resets only +0x58 and erases expired entries with release/file cleanup.
**Its comparison is unsigned now < expiration** (`008fd27f–008fd287`), unlike individual reads.
For ordinary pre-2038 positive times the tests agree; sign-bit-set timestamps must preserve
this distinction. No size-based eviction was established by this maintenance body.
RemoveCachedData erases a matching record with RAM release and attempted file removal and
returns true even on absence (`008fd69c`). File removal helper returns false for an empty
filename, otherwise attempts deletion and returns true regardless of deletion success (`008fc669`).

Persistence inventory: writer `008fc9b1` rebuilds section `Cache Entries` with sequential
`CacheEntry%05u` keys. Its value format is `%s,%s,%u,%u,%u,%d,%d,%d`: key, filename,
length, attribute, lifetime, creation, access, expiration. It attempts to materialize RAM-only
entries first and omits those still lacking filenames. Writer returns true without aggregating
individual write failures. It deletes the old INI file before rebuilding it. It does **not** call
the public file getter: no per-read expiration test, counter increment or access-time update occurs
(`008fc9b1`, direct reserve/write calls at `008fca37–008fca77`). Even an expired named record
can be written. Enumeration is native hash-table order: ascending bucket index, each bucket's
linked chain from head to tail (`0097c081`, `008fc653`, `008fc5e4`); it is not byte-key order.
The hash starts at zero and, for each key byte interpreted as signed 8-bit, doubles the previous
32-bit value then XORs the sign-extended byte, wrapping at 32 bits (`008fc450`). Bucket number
is unsigned hash modulo bucket count (`008fc477`). File ordinals follow this traversal and are
dense only over records with filenames.

### 6.4.1 File admission, naming and persisted entry acceptance (Q11/Q15 closure)

The source-file overload (`008fd76c`) rejects an empty key without effects; otherwise counters,
periodic maintenance and destructive same-key removal occur just as for byte admission, before
the source is inspected. It takes the source basename, not a RAM hold; sets attribute from the
byte argument, substitutes lifetime -1 and computes creation/access/expiration as for bytes.
With copy requested, reserve a generated cache filename then copy source to directory+filename
with overwrite enabled. Without copy requested, compare the lowercased source directory with
the lowercased cache directory; differing directories force copying. Equal directories retain
the basename and do not copy. Source is never moved/deleted by admission. Copy failure returns
false after old-entry removal, without undoing any reserved file. A reservation failure falls
through to the file-length check using the last candidate name, rather than guaranteeing false.
Finally obtain length from directory+stored filename: zero or 0xffffffff rejects; other lengths
insert the file-only record and return true. These failure paths do not restore the old record.

Filename and full-path helpers (`008fcd49`, `008fceeb`) start with the current 32-bit time,
printed as signed decimal. Take the suffix from the key's final dot only when that suffix,
including dot, is 2..8 bytes long. Remove every suffix byte except ASCII letters, digits and dot,
then append `.cache` (`00b1614c` → `00accc9c`). Format candidate as `%d%s` (`00acd40c`).
Thus key `a.html` at time 100 starts with `100.html.cache`; absent/long extensions give
`100.cache`. Try up to 10000 candidates, subtracting one from the 32-bit time after each failure.
Each attempt opens write-only with CREATE_NEW and shared-read, then closes immediately on success
(`00919b00`, disposition mapping `00919b4c–00919b58`). This reserves an empty file and refuses
existing names; it is not an existence-only check. On exhaustion return false and leave the final
attempted candidate in the output. The full-path variant takes an optional directory (null means
the cache directory), concatenates directly, and returns the full path; the name variant returns
only the basename while reserving in the cache directory.

The INI loader (`008fdb0d`) ignores empty values and values with fewer than seven commas.
Split from the **rightmost** seven commas: the remaining prefix is the key, followed by filename,
length, attribute, lifetime, creation, access, expiration. No quoting, escaping or trimming is
applied by this callback. A key may contain commas; a filename cannot round-trip a comma.
The entry label is not used as a key or a `CacheEntry%05u` validation test. All six numbers use
the wrapping decimal conversion in §3.2; nondigit/empty fields become zero. Length must be
unsigned <=4,000,000. Creation/access values greater than unsigned now are clamped to now.
Expiration must be unsigned >now. A rejected length/expiration attempts deletion of the named
file; other syntax failures simply skip. An accepted record requires directory+filename to
exist, but does not verify its actual length against the persisted length. The loader inserts a
file-only record; it does not read bytes or enforce the configured file/RAM capacities
(`008fdc21–008fdcb9`). Those settings have no size-eviction enforcement in this owner; the
periodic sweep is expiration-only.

Orphan cleanup (`008fc6e0`) enumerates files matching `*.cache` (`00b16150` → `00acd3bc`)
in the cache directory and removes a file only when no map record has a byte-for-byte equal
filename (`0043ab70`). It does not fold case or delete every unrelated file in the directory.
It returns true even when enumeration fails or a deletion fails.

RAM holders have a 16-bit reference count. Release decrements counts >1; otherwise it frees
the buffer pointer and holder with GlobalHeapFree (`008fc3c3`, calls `005e5620` at
`008fc3de`/`008fc3e4`). The no-copy byte overload transfers this allocator-compatible buffer
ownership; callers retaining access must hold a native reference. Clearing the map releases
its holds, not independently retained ones. Allocation failure and reference-count overflow
remain outside the qualified input domain.

### 6.5 Independent conformance vectors (PROPOSED / UNEXECUTED)

Fixtures supply valid records and deterministic clock/file replies; these are spec vectors, not
game observations. AL is the boolean channel; upper return bits are not asserted.

| ID | Input / setup | Required outcome |
|---|---|---|
| NFC01 | Live RAM record, expiry=101, now=100; data output null | true; no file read; access=100 |
| NFC02 | Same, now=101 | false; RAM released, file removal attempted if named, map entry erased |
| NFC03 | Existing key K; add K with null bytes and size=10 | false; old K removed before rejection |
| NFC04 | Existing K; add empty key with valid bytes | false; K and counters unchanged |
| NFC05 | Add bytes at now=100 with lifetime=-1 and default=432000 | creation=access=100; expiry=432100 |
| NFC06 | Add bytes with lifetime=-2 | expiry=2147483647 |
| NFC07 | Live RAM-only K; file retrieval with null output | false; no filename reservation/write; access updated |
| NFC08 | Live RAM-only K; output supplied; reserve succeeds, write fails | false; stored filename empty; output equals directory; access updated |
| NFC09 | Counters +0x54=9, +0x58=40; data miss | +0x54=10; maintenance runs and resets +0x58=0; false |
| NFC10 | now=`80000000`, expiry=`7fffffff` | individual presence sees live; periodic maintenance removes the same record |
| NFC11 | Set directory to empty | false; stored directory remains empty |
| NFC12 | Init; create-directory and INI-read both fail | initialized=true, result=true; second Init makes no retry |
| NFC13 | Reserve key `a.html`, now=100; first candidate exists | reserve empty `99.html.cache`; leave existing `100.html.cache` untouched |
| NFC14 | Source outside cache, copy=false; forced copy fails | false; old same-key entry removed; source preserved |
| NFC15 | Writer on expired named record, counters 9/10, access 50 | record emitted; counters/access unchanged |
| NFC16 | Loader value `a,b,f.cache,3,1,-2,200,300,500`, now=100, file exists | key `a,b`; filename `f.cache`; creation/access=100; expiration=500 |
| NFC17 | Loader length 4000001 / expiration equal now | reject and attempt named-file deletion |
| NFC18 | Orphans `a.cache`, `A.cache`, `keep.txt`; only map filename `a.cache` | preserve `a.cache` and `keep.txt`; attempt deletion of `A.cache` |
| NFC19 | Writer fixture bucket 0 contains key Z, bucket 1 contains A | ordinal 00000 is Z, 00001 is A; no lexical sorting |

## 7. cRZNetSocket and cRZNetCommandGenerator

### 7.1 Native socket contract and exclusions

Use [native transport](network-transport.md) sections 2–3 for construction, close/bind/connect,
stream state and transfers; sections 5–6 for error translation/readiness/available-byte count;
section 9 for the per-body inventory, interface slots and unsupported methods. Those sections
also hold the independent proposed socket vectors. This spec adds no second interpretation of
the socket owner or transport errors. In particular, normal success need not clear stale errors,
and shutdown/close state mutation may precede a failing raw OS call. Name lookup has its own
bounded queue/cache/ownership contract in that document's section 7.

### 7.2 Command-generator inventory, state and messages

*[compat]* Native primary table `00ac0130`, slots 0–6: QueryInterface `0081e800`, folded
AddRef `004664a0`, Release `0081e860`, Init `0081ea30`, Shutdown `0081e8c0`, destructor
`0081e9e0`, FinalRelease `0081e880`. Secondary message-target table `00ac0120` at object+4;
DoMessage entry `0081f450` receives that adjusted pointer and subtracts four before calling
owner helpers. QI accepts IDs 1, `2bbda365`, `2bbda368` returning the primary pointer;
ID `452294aa` returns object+4. Successful QI AddRefs the primary object; failure returns
false without clearing the caller's output (`0081e800–0081e85b`). Null/invalid objects are outside
the domain.

Nonvirtual inventory: constructor `0081e990`, PostIdleMessageToSelf `0081e930`, ExecuteCommand
`0081eb30`, DecodeHTTPCommandLine `0081ebd0`, ReadCommandInput `0081ef00`, WriteCommandOutput
`0081f180`, CheckForCommands `0081f320`. Constructor initializes active byte +8=false,
reference count +0xc=0, embedded stream socket +0x10, timer +0x70 with mode 4,
message-server pointer +0x88=null (`0081e990–0081e9d1`). It performs no drawing or command
execution. The socket constructor/state contract is the linked transport contract.

Init sets active before trying socket creation. On socket-create success it prepares an address
from constants `0xc364` with byte swap and zero host argument, attempts bind, and on bind
success calls Listen(5) (`0081ea41–0081ea98`). The address/port interpretation is Q12; do not
confuse swapped register contents with a host-order port literal. It starts the timer, acquires
message-server service class `04fa845b` / IID `652294c7` when absent, subscribes object+4 to
message `0bbd947b`, and posts that message. Returns true even after socket/bind/listen failure;
active remains set, so another Init makes no retry until Shutdown.

PostIdleMessageToSelf does nothing without a message server. Otherwise create/retain a standard
message, set type `0bbd947b`, post through server slot +0x10 with flag 0, and Release it
(`0081e930`). Allocation failure is outside the qualified domain.
DoMessage always returns true. An unrelated type causes no timer/poll/repost action. For
`0bbd947b`, poll only if unsigned timer elapsed **>250** (250 itself does not poll); restart
the timer before CheckForCommands. Then repost the idle message whether or not polling ran
(`0081f450–0081f492`). Timer units are supplied by the timer dependency; no extra sleep is implied.

Shutdown, only if active, clears active, unsubscribes the adjusted target and Releases/nulls
the message-server hold. Only if listener stream-state==3 does it call socket slot +0xec,
Shutdown(2), then Close. Returns true; it does not wait for pending idle messages (`0081e8c0`).
The shown DoMessage body itself does not test active, so callers must not assume late messages
are rejected by that flag.

### 7.3 Accepted-command IO boundary and failures

CheckForCommands requires listener state 3 and read readiness with timeout=0. It constructs a
temporary stream socket and calls listener Accept(temp, accepted-byte-output). Only a true
return **and** nonzero accepted byte enter the command path. It sets send and receive timeouts
to 5000, reads command input, and if reading succeeds executes and writes the result. It then
Shutdown(2)/Closes the accepted socket. At most one accept is attempted per poll; there is no
drain-until-empty loop (`0081f320–0081f406`). The export pseudocode deletes this branch as
unreachable; the instruction branch at `0081f36f–0081f383` is the evidence for this contract.

DecodeHTTPCommandLine starts after the first `=` if any, otherwise at the start; appends decoded
text to the existing output, converts `+` to space and valid two-digit percent escapes to a byte,
then trims the resulting output (`0081ebd0`). It does not split `&` parameters. Malformed/truncated
escapes and the request-line/body grammar remain Q12, as do ExecuteCommand's service-result
mapping and WriteCommandOutput's exact concatenation. Output transmission calls stream slot
+0x10c repeatedly, advances by the returned byte count and ignores the boolean result
(`0081f180`); zero progress has no proven terminating failure path. A replacement must not claim
this owner guarantees completion after a failed send.

### 7.4 Independent dispatch vectors (PROPOSED / UNEXECUTED)

| ID | Fixture | Required outcome |
|---|---|---|
| CMD01 | Unrelated message type | true; no poll/repost |
| CMD02 | Idle message; elapsed=250 | true; repost, no restart/poll |
| CMD03 | Idle message; elapsed=251 | restart, one poll, repost; true |
| CMD04 | Init; socket creation fails | true and active set; second Init does not retry |
| CMD05 | Listener state !=3 | poll does not check readiness or accept |
| CMD06 | Accept returns true but accepted byte=0 | no read/execute/write |
| CMD07 | Valid accepted connection; ReadCommandInput false | no execute/write; shutdown(2), close |
| CMD08 | Decoder input `cmd=hello+world%21`, empty output | `hello world!` |
| CMD09 | Decoder input `a=b&c=d`, empty output | `b&c=d` |
