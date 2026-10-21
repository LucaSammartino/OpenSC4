| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-url-contracts4 (RE: cRZURL contract gaps, web-network §2, Q4-Q6) | `re-url-contracts4` / `.claude/worktrees/re-url-contracts4` | ready at `6c817a02` (spec only; Q4 DIFFERS, Q5/Q6 MATCHES; full cRZURL method inventory) | 1. adjust `engine/core/src/rzurl.rs` ChangeURL chain (Q4); 2. add GetUser/Password/Params/Query/Fragment, setters, SetFullURL, non-lazy GetPort, ChangeURL(URL) overload |

## Report
- Static RE only (re_query.py asm + decomp/cRZURL.c); REA not needed.
- web-network.md §2.1: method inventory of every cRZURL method (ctors: no parse at construction; operator= copies all
  incl. parsed/type; GetFullURL/GetPort not lazy; lazy getters user 00823780, password 008237f0, params 00823940,
  query 008239b0, fragment 00823a20; setters 00822000..00822150 leave parsed/type/full alone; SetFullURL 00821fc0
  (text, invalidate)). cRZURL has no IO, cache or error reporting beyond its booleans.
- §2.3 step 5 (Q6 MATCHES): unrecognised `xyz://` scheme kept.
- §2.4 port (Q5 MATCHES): classifier is isdigit (mask 4); whole run erased; first 15 digits copied; atol 009ef7c2 wraps
  mod 2^32, no 65535 check; 16+ digits undefined in original.
- §2.7 (Q4 DIFFERS): the chain tests N's own params, then query, then fragment; a non-empty N field is never
  overwritten. Also: entry clear omits user/params/query/fragment (stale on early returns); "base empty" parses with
  the object's own allowRelative; result keeps N's type (relative N stays type 9); overload 00823ed0 passes the base
  object's stored full text or null.
- Questions: web-network Q4-Q6 moved to docs/specs/questions/answered/web-network.md; none filed. No ABI rows.
