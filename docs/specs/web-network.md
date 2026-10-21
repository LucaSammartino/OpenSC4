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
This revision specifies **`cRZURL` completely** (section 2) and **`cRZCookieManager`** (section 3); the other owners
are outlined in section 4 with their measured size and stay unspecified. No live network request was made.

Tags: *[inferred]* = role named by behaviour, not by symbol. *[quirk]* = observed behaviour that looks unintended;
an implementation must reproduce it for byte-exact parity.

## 1. Owner sizes (tools/coverage.py --owners, 2026-10-21)

| Owner | Bytes | Specified here |
|---|---|---|
| cGZWWWSystem (incl. the FTP client it reaches, ~30 KB) | 34049 | no (outline 4.1) |
| cRZHTTPClient | 15133 | no (outline 4.2) |
| cRZURL | 9498 | **yes, section 2** |
| cRZCookieManager | 8137 | partly, section 3 (parse, add, lookup, eviction; file format and capacity open) |
| NetworkFileCache | 6699 | no |
| cRZNetSocket, cRZNetStreamSocket, cRZNetCommandGenerator, cRZNameLookup | 11671 | no |
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
5. If a type was set, stop (success).
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
- If c exists and c < s (or no `/`): the port is the run of characters after c for which the C classification
  call at 008228b9 (one-argument CRT classifier 009ef61d; decimal digits *[inferred]*) holds, at most 15 kept
  (a longer run overflows a 16-byte buffer in the original: undefined, do not rely on it); if that run is non-empty, port = `atol` of it.
  server = rest[0,c); erase rest[0, c + run length + 1).
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
    - N path empty: take B's path; then, only if N's params are empty, B's params; only if those are empty too,
      B's query; only if that is empty too, B's fragment (a chain, each step inside the previous one).
    - N path not starting with `/`: start from B's path, cut after its last `/` (keep the `/`; if B's path has no `/`
      keep it whole), append N's path, normalise (2.8), and use that.
    - N path starting with `/`: kept.
    Rebuild N's text (2.6), then copy N into this object (full, components, parsed, allowRelative). Return true.

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
with n items the first **n-1** are stored as `atol` values and entry n-1 is set to 0 *[quirk or list-function
convention, see Q1]*.

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

### 3.5 Eviction (008f9a92, called as evict(4, no domain filter))

First drop expired records (helper 008f9a07, returns the count removed *[inferred]*). While fewer than 4 were
removed and records remain, remove the record with the smallest expiry, counting a session cookie (2) as expiring
now *[inferred from the selection loop; the removal step was not re-read]*.

### 3.6 Persistence and settings (not specified in this revision)

WriteCookieDataFile / ReadCookieDataFile store records as `CookieData%05u` entries under a `CookieData` key with
`Binary` and `Admin` settings (strings at 00accca4, 00accdc0, 00a86de8, 00accdb0); SetShouldEncryptCookieDataFile
selects an encrypted form. The record layout on disk, the encryption and the store capacity set by Init are open
(Q2, Q3).

## 4. Owners not yet specified (outline only)

### 4.1 cGZWWWSystem (34049 B)

The service front end: Init/PostInit/Shutdown, cache and cookie usage switches, a default notification target, job
IDs mapped to HTTP or FTP client jobs (HTTPClientJobIDFromWWWSystemJobID, FTPClientJobIDFromWWWSystemJobID),
visited-URL set, URL handlers, document load status, job update notifications. About 30 KB of its bytes are an FTP
client reached only from it (00811c90..0081e760): commands `USER`, `PASS`, `ACCT`, `SYST`, `TYPE A`, `TYPE I`,
`PORT h1,h2,h3,h4,p1,p2`, `LIST`, `STOR`, anonymous login, and a directory-listing parser that recognises `<DIR>`,
`<dir>`, ` DIR `, Unix `d` mode lines and a `Disk size` trailer (00812de0).

### 4.2 cRZHTTPClient (15133 B)

HTTP/1.x client jobs, TimeStringToSeconds (HTTP dates), DoJobFinalization (resolves redirects with
cRZURL::ChangeURL), cookie send and receive through 3.3 and 3.4, disk cache through NetworkFileCache.
