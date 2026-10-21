# Answered questions: web-network.md

## Q4 (core-url4, 2026-10-21) — ChangeURL empty-path chain
§2.7, N path empty: "only if N's params are empty, B's params; only if those are empty too, B's query; only if that is
empty too, B's fragment". Is B's query copied over N's own non-empty query (so `?new` against `http://a/b?old` keeps
`?old`), or only when N's query is empty too? Same for the fragment. [literal: copied unconditionally once the previous
field is empty, so `?new` is replaced by B's query when B has one]

**ANSWERED 2026-10-21 (re-url-contracts4):** DIFFERS — each step of the chain tests N's own field: N params empty -> take B params; then N query empty -> take B query; then N fragment empty -> take B fragment; a non-empty N field is never overwritten (`?new` against `http://a/b?old` keeps `?new`). 00823d6c..00823deb; spec §2.7.

## Q5 (core-url4, 2026-10-21) — port digit run and atol overflow
§2.4 server step: with a run of more than 15 port characters, how many characters are erased from rest (the whole run
or the kept 15)? And for a kept run whose value exceeds 32 bits (or `LONG_MAX`), what port does `atol` give: wrapped
modulo 2^32, or clamped to `LONG_MAX`? [whole run erased; first 15 digits wrap modulo 2^32]

**ANSWERED 2026-10-21 (re-url-contracts4):** MATCHES — the whole digit run is erased (00822914 erases c+1+full run length), only the first 15 digits are copied; atol 009ef7c2 wraps modulo 2^32 with no overflow check; classifier is isdigit (mask 4). Runs of 16+ digits stay undefined in the original; use the first 15. Spec §2.4.

## Q6 (core-url4, 2026-10-21) — unrecognised `xyz://` scheme
§2.3 step 4/5: for a scheme found by `://` that matches no prefix (e.g. `gopher://h/x`), is the scheme field cleared
before steps 6/7, or kept? Kept means an allowRelative URL ends as type 9 with scheme `gopher`, server `h`, path `/x`,
and ChangeURL treats it as "N has a scheme". [kept]

**ANSWERED 2026-10-21 (re-url-contracts4):** MATCHES — kept: scheme is cleared only at the start of ParseScheme (00822187..0082219b); `gopher://h/x` with allowRelative ends type 9, scheme gopher, server h, path /x. Spec §2.3 step 5.
