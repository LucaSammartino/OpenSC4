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
