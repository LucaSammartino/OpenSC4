# UTF8IsStringValid0091016F: independent public draft review

## Status and reviewer

DRAFT, WITHHELD from admission and credit. Worker review by Codex in lane
`tools-coverage-utf8-qualification4`, 2026-10-23, independent of the contract writer and engine
author. ROOT has not reviewed or approved these artifacts. `review.approved=false` is intentional;
this is not a resolver-admissible qualification. Candidate entry `0091016F`, standalone
`UTF8IsStringValid`, 416-byte public attribution only. No ledger insertion, owner completion,
native verification, replacement or byte credit follows from this review.

The review finds complete correspondence to the **published normal-domain policy**, subject to
the explicit representation and provenance limits below. This is an analytical conclusion about
the whole supported policy, not a claim that finite fixtures cover all byte arrays or that a
current whole-main gate ran. No known valid normal-domain branch is knowingly omitted.

## Authority and lane binding

Read `docs/COVERAGE.md` exact-function protocol and `docs/specs/utf8-validation.md` sections
2, 3, 4, 5 and 6 before substantive production/assertion/log analysis. Public specification is the
behavior authority; original executable, RE output, SDK implementations and private sources were
not consulted. Section 1 supplies public attribution only. `strings-language.md` only separates
the standalone helper from its neighboring owners; `foundations.md` defines specification tags.

The supplied `build-runs/evidence-run4/coordinator-utf8-qualification-inherited-phase-reservation.json`
records the actual 08:23:22 reservation: parent `tools-coverage-functions4` used 1, frozen limit 1,
exclusive child `tools-coverage-utf8-qualification4`, child ceiling 2, family ceiling 3. The copied
eligibility and explicit TOOL brief record root peer scope approval, fresh full coverage-accounting
TOOL GO after reservation and both dependencies on main at `a14ca1f6`. These are retained root
receipts, not checks rerun by this worker. Current coordinator state read during this review agrees:
parent rounds/limit 1/1 and exclusive waiting child; child rounds/limit 1/2, role tools, same manifest,
family-before 1/ceiling 3. This invocation is logical 2 of 3, child 1 of 2, maxRounds 1.
Manifest is `run4-tools-coverage-utf8-qualification4-r1-manifest.json`, base
`a14ca1f65a37a5c0772cd4f0a442bc10cef5350e`; detached worktree was reused without moving refs.
The lane guard allowed the reads and reported within budget. Runtime STOP/park/cap enforcement is
the guard's authority, not a fabricated global process census or a new admission check by this lane.

## Independent literal expectations first

The first phase of `build-runs/audit-utf8-qualification.py` parses only public contract table
literals and assertion `case!` data. It neither imports Rust nor invokes a decoder, production
model, tests or build. It compares exact bytes, count and expected Boolean: **92 public rows
expand to 107 identical cases** (V01–V44, T01–T15, B01–B15 each /7F and /C0, S01–S18).
Expected results come from the public ledger, never from evaluating production. The mechanical
comparison passed before the source-provenance phase. It establishes transcription equality,
not independently executed native parity or exhaustive input testing.

Semantic independence rests on the prior public contract's own literal derivations and the
following separate reasoning. V rows exercise lead and lower-bound extrema; T rows name every
missing-length state (1+2+3+4+5=15); B rows name every continuation position for widths 2–6
(1+2+3+4+5=15), each with the adjacent rejected values 7F/C0; S rows contrast counted backing
storage, zero, stopping suffixes, deferred bounds and mixed advancement. Synthetic counts are
separate assertions, not giant allocated fixtures. Their adequacy is checked against the full
decision structure below, not inferred from 107 as a count.

## Complete decision and effect correspondence

Production P is `engine/core/src/utf8_validation.rs`; assertions A are
`engine/core/src/utf8_validation/tests.rs`; contract C is `docs/specs/utf8-validation.md`.
All named symbols/text below are present in the bound files. Tests' `assert_reads` calls the
**same private `walk`** as public `utf8_is_string_valid`, with a raw-byte callback and offset
recording. There is no duplicate classifier/walker generating expected decisions.

| Decision/effect | Contract | Authoritative production | Independent assertions and scope |
|---|---|---|---|
| Count admission and unit | C §2: byte count 0..7FFFFFFF, readable span, not sentinel | `MAX_COUNT`, `check_count(count)`, `utf8_is_string_valid` calls `check_count(bytes.len())?` then `walk(bytes.len(), ...)` | `count_domain_is_distinct_from_false_without_allocating_gigabytes`: 0 and MAX_COUNT Ok; MAX_COUNT+1 and usize::MAX Err; FF Ok(false). These prove predicate boundaries, not actual max-size allocation. |
| Counted prefix and zero | C §2/§5.4: empty no read; embedded NUL ordinary; backing beyond count irrelevant | `walk`: `current = 0; while current < count`; public callback `bytes[offset]` sees exactly the slice | `public_107_literal_cases`: V01–V03, S01–S07; `no_reads_on_empty_and_no_continuation_reads_on_all_15_truncations`: V01/S01 [] and S07 [0]; ordered S02 [0], S03/S04 [0,1], S05 [0,1], S06 [0,1,2]. |
| Exhaustive unsigned leads | C §3 eight disjoint inclusive rows | `let lead = read(current)` is u8; match 00..7f =>1, 80..c1 or fe..ff =>false, c2..df=>2, e0..ef=>3, f0..f7=>4, f8..fb=>5, fc..fd=>6 | V02–V44 literals in `public_107_literal_cases`: both ends, invalid C0/C1/FE/FF and continuation leads; V19–V22, V24–V32, V35–V38 protect historical and non-scalar acceptances. |
| Total-width guard precedes all continuation reads | C §3, §5.2: remaining >= total width, otherwise stop after lead | `if count - current < width { return false; }` before `for position in 1..width` | All T01–T15 return false in public cases and exactly [0] in truncation test; S07 readable backing has excluded continuation but still [0]. |
| Ascending continuation checks; first failure exits | C §3, §5.3: 80..BF each position; no later read or bound test on failure | `for position in 1..width`, `read(current + position)`, `if !(0x80..=0xbf).contains(&byte) { return false; }` | `all_30_bad_continuations_stop_at_first_bad_position`: B01..B15 /7F and /C0 have literal prefixes [0,1] through [0,1,2,3,4,5]; public cases assert false. S10/S11 [0,1] ignore differing later suffixes. |
| Save first raw continuation without early lower-bound decision | C §3/§5.4: low but structural first continuation does not skip later checks | `if position == 1 { first = byte; }` only caches; `width >= 3` block is after complete continuation loop | S12 [0,1,2,0] versus S13 [0,1,2]; S14 [0,1,2,3,0], S15 [0,1,2,3,4,0], S16 [0,1,2,3,4,5,0] in `public_stopping_and_lower_bound_read_order`. |
| Actual lead re-read and four deferred minima | C §3 and §5.4 ordered traces: re-read lead, no re-read of first continuation; E0/A0, F0/90, F8/88, FC/84 only | `match read(current)` inside `if width >= 3`; constants a0/90/88/84, `_ => 0x80`; `if first < minimum { return false; }` | S12/S14/S15/S16 prove each re-read and deferred failure; S13 proves earlier bad continuation prevents it. V10–V18, V23–V34 and V41–V44 distinguish minima, accepted boundary, non-special next lead and overlong rejection. Successful re-read for every non-special lead is established statically, not by a dedicated trace fixture. |
| Exact success advance and full span, including next invalid lead | C §3: advance width, stop without advance on reject; succeed only at end | `current += width` is after all rejects; loop then `true`; every reject returns before the update | S17 widths 1,2,3,4,5,6,1 =22 returns true; S18 consumes 2 then rejects BF, trace [0,1,2]; S08/S09 [0,1] protect no suffix scan. V03 multi-ASCII; S06 continuation then zero. Internal consumed offset is not an output. |
| No additional effects or decoder restrictions | C §2/§4: AL Boolean; no writes, allocation, calls to foreign validator, normalization or scalar upper bound | Public returns `Result<bool, UnsupportedCount>`; walker only reads callback, mutates locals and returns bool; no unsafe, allocator, shared state or foreign routine | V19/V20 modern-boundary contrast, V21/V22 F5/F7, V24–V32 five/six-byte, V35–V38 surrogate-neighbor cases. Source inspection, not fixture count, establishes absence of writes/host substitution. |

## Universal argument beyond fixtures

For admitted count n, use the invariant 0 <= current <= n <= 7FFFFFFF. Base current=0.
If current=n, no byte read occurs and true is correct. Otherwise a lead read is in-span.
The eight public classes partition all 256 u8 values, including both rejected intervals. For a
rejecting class the return is immediate. Otherwise width w is 1..6; if n-current<w no
continuation is read and no advancement occurs. If the guard passes, each j in 1..w is
strictly less than w, so current+j<n and the addition cannot overflow. The loop visits each
such j once in increasing order. A failed continuation exits immediately. If all succeed,
`first` is exactly position 1 for multibyte rows; the width-1 empty loop never uses it.
Width 2 needs no exceptional check; widths >=3 re-read the original in-span lead after all
continuations and apply exactly the four minima or 80. Every continuation already lies in
80..BF, so the default minimum imposes no hidden rejection. A failed extra bound exits before
update. Otherwise current'=current+w<=n preserves the invariant and strictly increases current.
The natural variant n-current decreases, proving termination for all finite admitted spans.
Thus loop exit is equality, never overshoot, equivalent to C's AL=1 end equality; any reject
is before completion. No failure path advances or reads later data. This inductive argument
covers arbitrary concatenations and all interior byte values, not merely boundary fixtures.

## Native representation and normal-domain fit

C §2 explicitly bounds normal count to 0..7FFFFFFF with nonwrapping 32-bit pointer+count and
readable positive span. On that domain residual counts remain nonnegative and signed-32
representable, so signed length guards agree with the remaining-width predicates stated by C
§3. This relies on the public contract's guard semantics, not an assertion about machine
instructions independently inspected here. No extra restriction to low-half pointer addresses
is introduced. A safe slice represents bytes and count, not native pointer identity; translation
is offset=current-pointer-base. Nonwrap ensures equality/order along that span corresponds to
offset order. No public branch depends on absolute pointer bits apart from excluded upper EAX.

The native two-stack-word cdecl-compatible entry is not exposed by this portable Rust API.
Rust usize above MAX_COUNT returns UnsupportedCount; it does not fabricate native false.
Native negative/high-bit counts, wrapping ends and pointer validity are not specified by the
normal contract. This is a pre-existing public domain, not one invented to hide an unimplemented
known valid branch. Null-zero deserves a separate boundary: C says no null validation and puts
null use outside supported contract, even for zero; the portable empty slice establishes the
normal readable-backing empty case. Separately, the documented native end=pointer+count and
final equality imply that at count zero end=start even at address zero, with no byte read:
that pointer-independent AL=true/no-read branch is represented by an empty slice's true/no-read
behavior, rather than treated as a fabricated false or hidden missing branch. This is a public
entry/exit arithmetic inference, not a native null call or certification of a raw-pointer API.
The contract still excludes null-pointer use as an API domain; no null validation or native
fault behavior is invented. Any extension of the public input interface requires renewed
whole-function review, not silent reclassification.
UAF, unreadable/invalid pointers and concurrent mutation cannot be represented by a valid shared
slice. The actual lead re-read is nevertheless implemented; mutation between reads is not proven
or emulated. bool represents AL, never the pointer-derived upper EAX. No native fixtures ran.

## Source and retained execution provenance

The read-only analysis verified seven raw R2 log byte lengths and SHA-256 against `terminal.json`,
literal terminal PASS lines, all five named UTF8 PASS lines and all 26 Rust summaries totaling
5504/0/0. See [retained public validation](utf8-validation.txt) for exact argv, times and digests.
Start/end source maps are equal, 1327 files; terminal binds stable engine tree
`93ec65884a6ec407ea6fb403583eeece2d3d37e1`, head `00ba5deee290d3725c1d1144ca7fa1d9470bf6a1`,
aggregate `c3e22cff564280e4bbc4f3606c63a20c4b84d48fe3e012848501b79654b6a7dd`.
Git independently resolves that head's engine tree to the receipt, and both exact UTF8 Git
blobs hash to their actual start/end source-map values. Current UTF8 files have CRLF; converting
only CRLF to LF makes them byte-identical to those gated Git blobs. Raw mismatches are line
endings, not a source change. The official `coverage.py.text_sha256` also handles UTF-16 BOMs;
these inputs are UTF-8, so no UTF-16 conversion is applicable.

Important distinction: **40 engine files differ** between R2 head and this base. `lib.rs` has
seven additive unrelated module declarations (effects_parser, local_grid_helper, rigid_body_2d,
game_of_life, video_preferences, network_transport, text_decal_occupant); `pub mod utf8_validation;`
is unchanged. Relevant Cargo/config text is unchanged. Cargo.lock is Git-identical after LF
normalization but is absent from the 1327-file start/end maps; no raw runtime lock stability is
invented. This qualifies retained execution for unchanged UTF8 production/assertions with explicit
integration limits; it does not prove this whole current engine was run at the old gate.

The first analysis attempt passed literals/rawlogs/UTF8 source matches then failed a too-strong
raw Git-vs-worktree check on lib.rs. Investigating established CRLF checkout hashes plus actual
additive module changes. The corrected analysis distinguishes these facts; it does not overwrite
the failed observation with a fabricated all-engine stability claim.

## Falsification and remaining proof limits

This is a static challenge of the proposed correspondence, not mutation execution or a new test
round. Candidate mistakes were checked against independent contract consequences:

| Candidate defect | Discriminating evidence |
|---|---|
| Modern RFC decoder rejects historical forms or surrogate encodings | V20–V22, V24–V32 and V36/V37 explicitly expect true; inspected P has no scalar-limit branch. |
| Treat zero as end marker or ignore a failing tail | S03/S04, S17/S18 distinguish full-span consumption; `current += width` and invariant explain arbitrary concatenations. |
| Guard just one remaining byte or read accessible backing past count | T01–T15 and S07 must read only the lead; actual P checks total width before loop. |
| Continue after first malformed continuation | B paired traces and S10/S11 include both valid/invalid suffixes, same early prefix reads; P immediately returns. |
| Test E0/F0/F8/FC lower bound too early, or omit actual lead re-read | S12–S16 give exact deferred traces; S13 specifically forbids the re-read after bad position 2. |
| Wrong bound such as E0/9F or F8/87, or impose bound on all leads of width | V10/V11, V15/V16, V23/V24, V28/V29 and V13/V18/V26/V31 separate each boundary and special/default branch. |
| Advance one byte for multibyte success, overshoot, or silently skip a next lead | S17 mixed widths and S18 trailing BF reject; invariant establishes all widths/interior positions universally. |
| Map count overflow to native false or accept all usize lengths | `check_count` synthetic boundaries separate Err from Ok(false); public function calls it before walker. |

These are discriminators and dataflow reasoning, not assertions that every possible source
mutation is caught by finite tests. Successful width>=3 default-lead re-read is not separately
traced in A, maximal supported buffer is not allocated, and all interior bytes/long concatenations
are not enumerated. The universal argument supplies these obligations using the actual match,
loop range, guard, update and lack of other state. No generated expected result or production
execution was used in this audit. C §3's completeness and its representation boundary are
public evidence relied upon, not independently re-derived native facts by this worker.

The draft JSON's `normal_domain_complete=true` records the worker's whole **published-policy**
conclusion above. `owner=null` is an intentional unresolved exact-Attribution binding, not a
fabricated owner string, and `review.approved=false` keeps the draft inadmissible. The supplied
public entry name/address/416-byte body does not certify the resolver's current owner, exclusions,
folded-name ambiguity or current export/artifact digest tuple. ROOT must bind those separately
before any proposed ledger claim. No contract/implementation/test branch is declared outside
normal solely to fit Rust. If independent review finds a valid missing branch or a defective
public normal-domain premise, the entire entry must remain withheld and completeness become
false; no fraction of the 416 bytes may be admitted.

## Normalized dependency manifest

All digests below use unchanged `coverage.py.text_sha256` semantics. These UTF-8 files need
only CRLF-to-LF replacement; no whitespace or other text rewrite. The lane report separately
hashes all three finished draft proof files, including the qualification JSON itself.

| Bound dependency | Normalized SHA-256 |
|---|---|
| docs/specs/utf8-validation.md (entire contract, including §2/§3/§5) | 590d1b7f02ae5a5768c733c001760c72f8276983efec71d7b74b9a89c2008ab0 |
| docs/specs/strings-language.md (standalone separation) | 7d588e5a05a837ec47f3165b0f9f73a861dd0f4a8d618885bf43c37511e5c82b |
| docs/specs/foundations.md (tag definitions only) | 4f265ca6dace78239316c038961aa4f63fb8d50318ccc21624fe7d5ac930e03a |
| docs/COVERAGE.md (strict admission protocol) | 0b8815f9c6e45ff5c870b190a90b3720763c5fdf9b52e8dce01d01c019bc2a97 |
| engine/core/src/utf8_validation.rs (`MAX_COUNT`:11, `utf8_is_string_valid`:23, `check_count`:28, `walk`:39) | 2cb015fce52537a91a0baaa6640430b55c6a0ed2d1d6be3bb7beee5c7957ae19 |
| engine/core/src/utf8_validation/tests.rs (`case!`:13, `CASES`:23; test/assertion symbols:142,154,168,178,205,229) | 58cdcc027c6b6db5bff6f013df82cb2cb614261e712093f330b305101e05f714 |
| engine/core/src/lib.rs (current module integration) | b6b45d9aa518358a3192dd684ffd00241c191d78e7ac20c7757433b61bcada99 |
| engine/core/Cargo.toml | 0e1083efd788603ca79452109ee662715660c0d42ba18bddaa441175bfd72343 |
| engine/Cargo.toml | 79103248f6c407f6deb3183f515b7aecc9aa9ccdc696576e4011abaf7b9e1e43 |
| engine/Cargo.lock (Git equality only, absent raw snapshots) | 840cf5fb3d86f24cfe24bc111a89b04af598ab5c9fa8fe0ceb2279146f3240d5 |
| engine/.cargo/config.toml | 67de482f508bbc47907ba7ed8e24c9d7961002ba10ff8b2b77f845911f9b1817 |

No foreign implementation dependency exists in the authoritative walker. Cargo integration hashes
are recorded for provenance and limits, not a claim to have audited unrelated workspace modules.
The retained validation binds raw logs/receipt custody and source stability separately from this
normalized text manifest. Changing any cited dependency or proof requires renewed review.
