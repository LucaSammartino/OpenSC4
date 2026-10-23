# Registry pilot whole-function review

## Independent ROOT review

Prepared by Codex /root/live_refill and independently reviewed by Codex /root on 2026-10-23. ROOT accepts the entire published normal semantics of the three functions below for specified and portable implemented stages. Native verification/replacement and whole-owner stages remain unqualified. The unchanged strict resolver must still validate identity, attribution, body boundaries and every evidence reference before numeric admission.

ROOT independently read public contract section3.2, the complete production accessors and record/lifetime helpers, the actual g6/g7 and g8 assertions, and all source-matched provenance. ROOT's read-only provenance audit at07:56:04 matched eight raw logs, terminal SHA, both source files at the historical gate and current main, stable engine2226d3b6, report-only head movement and 5388/0/0 in23 summaries. The retained tree-change warning and exact optional-statistics intervention remain disclosed below. No new checker/native/game run was made for admission.

ROOT explicitly accepts the universal direct-u32 setter dataflow proof: every successful path writes the parameter unchanged to the u32 context field, with no cast, mask, arithmetic, value branch, callback or other field write; null returns before mutation. Literal setter fixtures cover only null and zero. This is a source proof combined with actual assertions, not a claim that a nonzero or MAX setter fixture executed. The getter similarly projects the whole u32 field; full mask semantics use the exact bitwise formula. Valid live record tokens represent the public valid-readable-pointer domain; invalid/stale/foreign/fabricated pointers remain excluded by that contract. No known valid branch is omitted to obtain credit.

Published qualification files are registry-00473b50.json, registry-00473b70.json and registry-00473b90.json; retained validation is registry-pilot-validation.txt. ROOT approval does not alter attribution, owner baseline, denominator or the measurement parser.

## Whole normal domain and representation

game-data-registry.md section3.2 and section1 bind the three exact entries. Null and valid live records are normal. The public contract expressly excludes foreign/stale/fabricated pointers; the safe model's Option<EntryId>/records map is representation, not a new native subset. record() checks service lifetime and fetches the record, not node/leaf key-index membership. Thus still-live root remains accessible after Shutdown even though IsValidEntry(root) becomes false. Destroyed service, invalid handles and destructive reentry remain outside the valid-memory domain. None of these three functions calls a foreign primitive or host oracle.

| Entry | Entire published decision/effect | Authoritative source | Independent actual assertions |
|---|---|---|---|
|00473B50 CheckEntryFlags|Null false for every mask; non-null exactly(flags&mask)==mask over all32 bits, including zero-mask true for node/leaf; no mutation/ref/event calls.|check_flags -> record; is_some_and null gate and exact u32 bitwise formula.|g8 null+zero false; g6_g7 node+zero true, bit80000000 true, maskFFFFFFFF false against flagsFFFFFFFE.|
|00473B70 SetEntryContext|Null false/no writes; valid record arbitrary full u32 context write and true; no other field/ref/event effects.|set_context: alive/null early return, record get_mut, direct context=context assignment, Ok(true). No value cast/mask/arithmetic/kind restriction.|g8 null setter false; g6_g7 valid setter0 true and subsequent readSome(0), previous context33330000 observed.|
|00473B90 GetEntryContext|Null nativeword0; non-null exact stored32-bit context; no mutation/ref/event calls.|context -> record.map(e.context), field/result u32, no mask/truthiness/kind/membership check.|g8 contextNone ->None; g6_g7 nonzero33330000 ->Some(33330000), then write0 ->Some(0).|

Portable None versus Some(0) is richer internal shape; both encode nativeword0. This is implemented semantic coverage, not native layout/ABI wiring. The JSON decision strings are actual substrings of the bound contract/source/assertions; human review, not substring presence, establishes correspondence.

## Universal dataflow proof versus literal coverage

CheckEntryFlags is precisely the Boolean bitwise formula over u32 variables. Measured null/zero-mask/top-bit/missing-low-bit cases discriminate null-is-zero, any-bit, signed truncation and nonzero-only alternatives. There is no FP/locale/environment state.

For SetEntryContext, for every c in0..=FFFFFFFF and valid e, the only successful-path write is e.context=c, both u32; then Ok(true). For null, return precedes mutation. No cast, mask, arithmetic, value-dependent branch, callback or other record write exists. This is a universal source dataflow proof, not empirical execution of all values.

IMPORTANT TEST LIMIT: existing setter invocations cover ONLY0 and null. Nonzero33330000 is read from creation, not written through set_context. A constant-zero setter would pass these setter fixtures in isolation but contradict the current source proof. ROOT independently accepts the direct u32 assignment proof plus measured null/write/read assertions for this entire method. No setter(MAX) or high-bit setter test was run or claimed; no new test is requested here.

GetEntryContext directly projects a u32 field. Nonzero33330000 and zero/None are measured; no top-bit context literal is claimed. Source absence of casts/masks proves that case. Source signatures/complete bodies also prove no host/ref/event calls; these functions do not take a Host parameter. Invalid-handle safe errors are outside native valid-pointer inputs, not graceful-fault claims.

## Actual source-matched completed gate

The actual sole round2 canonical qbuild all ran05:18:46.891365 to05:26:33.200024BST2026-10-23, collector exit0/session65673. Measured5388/0/0 over23 Rust summaries, Clippy0, native actualRC0/4of4, Ct68/68. Both relevant fixture names are actual PASS lines. All eight raw log hashes plus terminal hash independently match retained qualification/summary.

Production normalized SHA256 f60200160d8c71741adda5f065f7d6bc30fea60b1a39300f803b95689af80ffd
Assertions normalized SHA256 c8584cea5020ab7df0d08b60e1db174ee6dee74538ce317d3751dcea203cea4e

Both equal gate-before/gate-after raw hashes and normalized git objects at260d6f99. Current checkout CRLF raw differences are only line endings; docs/COVERAGE.md explicitly normalizes CRLF toLF. Current whole main engine/lib lists have later additions; this does NOT claim the entire current main ran the historical gate. Exact local record/token/type/helper dependencies reside inside unchanged bound mod.rs.

The raw tree-change warning is retained verbatim. HEAD260d6f99 ->3fc08ce1 was operational/report documentation while engine2226 and all three recorded source hashes stayed equal and clean. Earlier unformatted canonical results do not qualify final formatted source. Prior21evidence files, AccessDenied/format/patch/cwd errors and failed/incomplete attempts remain archived and unchanged.

ROOT interrupted ONLY read-only stats27548 during final pipeline: action05:24:16.413447, actualexit15 by05:24:18.018816; samefc22544+daemon8408 survived. Cache stats unavailable/causeUNKNOWN. Earlier35076 action belongs7bab/4f8e old source, not final source. Neither interruption is PASS evidence; actual downstream terminal is.

## Admission limits

Only specified/implemented are approved for resolver admission. Original method executions0; nativeverified/replaced/shadow/resync/default/ABI and other45inventory bodies/GDR1–5 stay unchanged. No factory/layout/whole-owner promotion. Fixed denominator/exclusions/current Attributed owner/folding/shared/body-overlap checks remain ROOT's normal resolver step at activation. This preparation read no original/private bodies and changed no attribution.

The supplied32/25/19 metadata is not promised numeric admission or rounded headline movement. No new engine test/build/native/game/worker/source feature ran for this publication. Numeric admission is conditional on the unchanged strict resolver, audit and tests; no rounded headline movement is promised.


