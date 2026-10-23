# Effects text parser questions

## EFP1 — Foreign switch-reader language and consumption

In `effects-parser.md` §2/§4, what exact contract does `0067A7D0` expose for
dash/case/delimiters, aliases, duplicates, missing/empty values, unknown options,
terminators and token consumption? Supply individual lookup outcomes and scratch
indices, not a host-produced final parser result. Present owner rules do not prove
these generic reader behaviors.

## EFP2 — Rotation instruction/store semantics

In `effects-parser.md` §2/§6, establish the x87 operation and binary32 temporary/store
order for all five rotation branches of `005989D0`; distinguish their key order
from text order. Current supported domain requires those lookups absent.

## EFP3 — Error transport and lifetime outside normal domain

In `effects-parser.md` §3, identify the actual error transport of `009EF84F` and
cleanup/unwind behavior of the select/LOD/prob errors. Do not assume false-return,
transaction rollback or resumed parsing from the misleading export name.
