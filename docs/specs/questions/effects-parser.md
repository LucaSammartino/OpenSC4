# Effects text parser questions

**EFP1 — Foreign switch-reader language and consumption**

In `effects-parser.md` §2/§4, what exact contract does `0067A7D0` expose for
dash/case/delimiters, aliases, duplicates, missing/empty values, unknown options,
terminators and token consumption? Supply individual lookup outcomes and scratch
indices, not a host-produced final parser result. Present owner rules do not prove
these generic reader behaviors.

**EFP2 — Rotation instruction/store semantics**

In `effects-parser.md` §2/§8, establish the x87 operation and binary32 temporary/store
order for all five rotation branches of `005989D0`; distinguish their key order
from text order. Current supported domain requires those lookups absent.

**EFP3 — Error transport and lifetime outside normal domain**

In `effects-parser.md` §3, identify the actual error transport of `009EF84F` and
cleanup/unwind behavior of the select/LOD/prob errors. Do not assume false-return,
transaction rollback or resumed parsing from the misleading export name.

**EFP4 — Foreign registration identity and ownership**

In `effects-parser.md` §5.1/§5.2, establish `P+90` slots `+44` and `+48`'s matching
interface contracts: named validation, duplicate/collision/replacement behavior,
null-name identity/index selection and reference transfer of the supplied object.
The owned bodies append locally; neither generates an anonymous name or counter.
Do not claim a foreign counter boundary from the raw returned-index ledgers.

**EFP5 — Final lifetime and shared reference operations**

In `effects-parser.md` §5.5/§8, positively identify destructor/refcount-zero cleanup
and held-interface release order. Linked callback `0059EBE0` is not yet proved a
destructor; shared AddRef/Release delegation cannot certify parser-exclusive
lifetime behavior. Distinguish this class's factory from the separate file queue.

**EFP6 — Command-library dispatch policy**

In `effects-parser.md` §6, establish foreign root/nested registrar duplicate,
lookup/case, descriptor ownership and block-hook dispatch policy. Current proof
is the owner's ordered literal-to-descriptor registration, plus the concrete
`sequenceEffect` normal handler, not all root command semantics.
