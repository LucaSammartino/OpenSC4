# Open questions: web-network.md

## Q1 (re-webnet4, 2026-10-21) — cookie Port list length
ParseCookieText (008f94f9) stores n-1 of the n comma-separated `Port` values. Does HTTPMessage::GetSeparatedStringList
return the item count plus one (so all values are kept), or is the last port really dropped? Needs a read of
HTTPMessage::GetSeparatedStringList; spec section web-network.md §3.2.

## Q2 (re-webnet4, 2026-10-21) — cookie store capacity
AddCookie evicts when the store holds at least the capacity (manager +0x10). What value does Init or the constructor
set? Spec section web-network.md §3.3.

## Q3 (re-webnet4, 2026-10-21) — cookie file format
Write/ReadCookieDataFile (008f7784, 008f9e2b): byte layout of each `CookieData%05u` entry and the encryption chosen
by SetShouldEncryptCookieDataFile. Spec section web-network.md §3.6.

<!-- from: core-url4 (2026-10-21) -->
## Q4 (core-url4, 2026-10-21) — ChangeURL empty-path chain
§2.7, N path empty: "only if N's params are empty, B's params; only if those are empty too, B's query; only if that is
empty too, B's fragment". Is B's query copied over N's own non-empty query (so `?new` against `http://a/b?old` keeps
`?old`), or only when N's query is empty too? Same for the fragment. [literal: copied unconditionally once the previous
field is empty, so `?new` is replaced by B's query when B has one]

<!-- from: core-url4 (2026-10-21) -->
## Q5 (core-url4, 2026-10-21) — port digit run and atol overflow
§2.4 server step: with a run of more than 15 port characters, how many characters are erased from rest (the whole run
or the kept 15)? And for a kept run whose value exceeds 32 bits (or `LONG_MAX`), what port does `atol` give: wrapped
modulo 2^32, or clamped to `LONG_MAX`? [whole run erased; first 15 digits wrap modulo 2^32]

<!-- from: core-url4 (2026-10-21) -->
## Q6 (core-url4, 2026-10-21) — unrecognised `xyz://` scheme
§2.3 step 4/5: for a scheme found by `://` that matches no prefix (e.g. `gopher://h/x`), is the scheme field cleared
before steps 6/7, or kept? Kept means an allowRelative URL ends as type 9 with scheme `gopher`, server `h`, path `/x`,
and ChangeURL treats it as "N has a scheme". [kept]
