# TextEdit measured owner attribution

Native Windows 1.1.641 analysis, measured 2026-10-22 by re-textedit4 round 2.
This is an attribution audit, not permission to promote whole-owner coverage.
The previous 134-entry direct-label inventory and its 19,415 bytes remain
valid. The `19999B` brief suffix denotes the coverage owner's measured byte
total, not a native class spelling.

## Reproducible inputs and result

Ran the main checkout's `tools/coverage.py` with explicit main-checkout
`ghidra/export/windows` and `bin/SimCity4.exe` inputs and **own-worktree**
outputs `build-runs/coverage-r2.md` and `build-runs/owners-r2.tsv`.
No tool, map or ABI table was modified. The exact-owner filter is equality
on the TSV owner column, not a substring match on labels. Address/size join
against the direct inventory is saved in `build-runs/attribution-r2.txt`;
its small reader is `build-runs/audit-textedit-r2.py` (ignored local evidence).

| Actual owner rule | Functions | Body bytes |
|---|---:|---:|
| name-high | 71 | 7,736 |
| name-medium | 56 | 11,630 |
| name-low | 12 | 104 |
| reach | 5 | 203 |
| neighbour | 6 | 326 |
| **Measured exact owner** | **150** | **19,999** |

All 134 direct inventory addresses occur in this owner output with the
same sizes. The following are **all sixteen** additional rows. Their rules
are analysis attribution, not a deduction that neighbouring bytes must be
editor methods. Native instructions/references separately establish roles.

| Address | Bytes | Actual rule | Primary evidence / role |
|---|---:|---|---|
| 0x00994F37 | 11 | name-medium | Text-vtable slot +0x18, adjusted SetArea |
| 0x00994F6E | 11 | name-medium | Text-vtable slot +0x50, adjusted SetFillColor |
| 0x009BC5CC | 21 | reach | Space/LF predicate; existing section 3 contract |
| 0x009BD0AF | 50 | reach | Preceding-LF correction; existing section 7 contract |
| 0x009BDDF2 | 76 | reach | Called by GetText at 0x009BE2FA; bounded byte copy from string |
| 0x009BDE3E | 25 | neighbour | Zero count returns null, otherwise requests count*60 bytes |
| 0x009BE9ED | 32 | reach | Initial line-start entry; existing layout call-site contract |
| 0x009BEA0D | 27 | neighbour | Destroys actions in a half-open range, 60-byte stride |
| 0x009BF518 | 24 | reach | Forwards range destruction to 0x009BEA0D |
| 0x009BFA74 | 30 | neighbour | Destroys action range, frees nonnull vector storage |
| 0x009BFA92 | 41 | neighbour | Constructs copies over a 60-byte-stride source range |
| 0x009BFABB | 40 | neighbour | Constructs count copies, 60-byte stride |
| 0x009BFAE3 | 163 | neighbour | Editor nondeleting destructor; existing section 10 order |
| 0x009BFB86 | 11 | name-low | Text-vtable slot +0x0, adjusted QueryInterface |
| 0x009BFB91 | 11 | name-low | Text-vtable slot +0x10, adjusted Shutdown |
| 0x009BFF47 | 11 | name-low | Text-vtable slot +0xC, adjusted Init |

The text vtable at 0x00ADFA68 and primary vtable at 0x00ADFBD0 are
independent native witnesses for interface adjustment and owner dispatch.
`re_query.py asm --bytes` confirms the helper widths/strides above;
`callers` ties 0x009BFA92 and 0x009BFABB to the action-vector insertion
body at 0x009BFE4B. Generated action helpers can be contract dependencies
even when coverage attributes them to library owners. Conversely, the
folded maximum setter and sound adapter have meaningful editor call-site
contracts but are not added to this exact owner's TSV total.

The 146-label / 19,908-byte name-search result is a **different set**:
it includes nested/generated/foreign/folded labels and omits unnamed reach
helpers. It must not be used as the owner denominator. TE8 attribution is
closed by the actual address join; TE8 method-contract residuals remain.

## Final-round proof disposition (3/3)

Attribution remains **150 / 19,999B**, with no owner-map changes or new
denominator. The direct inventory now has 97 P (10,136B), 37 P/B (9,279B)
and zero U rows. Its final reconciliation lists the exact bounded scope of
all 37 P/B rows. This is a method-contract count, not headline owner credit.

The sixteen additions have the following conservative dispositions:

| Address | Disposition and contract source |
|---|---|
| 00994F37, 00994F6E, 009BFB86, 009BFB91, 009BFF47 | P for the five 11B thunks: subtract 0xD8 from incoming T and tail-call SetArea, SetFillColor, QueryInterface, Shutdown, Init respectively. Final-round native instructions prove the entire adjustment bodies; callee bounds still apply. |
| 009BC5CC (21B) | P: exact space/LF byte predicate, text-edit-layout.md navigation. |
| 009BD0AF (50B) | P: preceding-LF insertion correction, text-edit.md section 7. |
| 009BDDF2 (76B) | P: bounded byte copy/count/start-failure branch, text-edit-adapters.md. |
| 009BDE3E, 009BE9ED, 009BEA0D, 009BF518, 009BFA74, 009BFA92, 009BFABB, 009BFAE3 | B: allocation/initial-line/action range destruction/copy/destructor roles in the measured table and sections 6/10. Section 6 now specifies copy/growth and absent local failure cleanup from instructions; allocator/string/base implementations remain foreign. Conservative attribution classifications are retained. |

This is **8 P / 202B** plus **8 B / 382B** among the additions, exactly
584B; direct and additional body sizes still sum to 19,999B. In particular,
the 163B destructor has an owner cleanup-order contract, but no full foreign
destruction/unwind promise. Metadata, role recognition and an adjustment
thunk do not certify a callee's unproved body. TE5/TE8 were answered on
2026-10-24 by the encoding/adapters and history-storage contracts, without
promoting the attribution count to complete-owner execution evidence.
