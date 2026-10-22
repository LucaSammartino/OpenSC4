| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-ftp4: portable WWW/FTP partial owner | Codex, core-ftp4 (detached reservation) | in progress; gates pending | Validate stable source and retain unsupported boundaries |

## Handoff
- Goal: published portable WWW/FTP first implementation; retained unsupported boundaries.
- Done: 0a57021f parser/path, 40adce68 state algorithms, 1f4111db review/Q10 milestones.
- Source: www module; report below reconciles all 91 C and exact 57 P/O rows.
- Next: finish quick diagnostics and baseline clippy/build; run final stable all after fixes.
- Baseline cargo: 5039 passed, zero failed, 21 result lines; clippy now running.
- Quick requests 70740/2570: one running unit tests, another had waited on build lock.
- fmt absolute-file session 75585; first relative invocation failed path lookup only.
- No game requests; no ABI/default/glue/map/coverage changes.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-ftp4-r1-manifest.json.

## Implementation

Adds engine/core/src/www/{listing,ftp,worker,http,service}.rs and URL/path helpers; reuses RzUrl and the same HTTP Resource representation in WWW completion. No ABI, plugin, default/replacement, shared coverage/map, game or URL/HTML-family edits.

Core owns destructive parsing, commands, partial transfer loops, type/login/dispatch state, serial queue decisions, cache policy/conversion, duplicate waiting-job fanout and ordered reference effects. Typed socket/file/time and foreign cache/cookie/handler service interfaces perform only external operations. Unpublished owned helpers return Unsupported. Native leaks remain visible as missing Release effects; Rust IDs do not silently repair native ownership. Native unsafe/uninitialized domains are unsupported, never deterministic garbage or asserted native false.

## Validation

Baseline all launched before source edits (session 51977): 5,039 cargo tests passed, zero failed, 21 result lines. Clippy in progress. Quick checks launched (70740, 2570); unit tests running after lock wait. Final stable all, clippy, full CTest and Rustfmt results pending. No game run requested (explicit lane prohibition). No external contact or live server tests.

Constructed vectors distinguish listing selectors/mutation/output preservation/unsafe domains, reply helpers, chunk short send versus command prefix resend, async state-4 reentry, rejected-login true, RETR write failure and empty success, ignored STOR sender/read failure, progress rounding, callback reentry, listing partial capacities, target/shutdown/file-wrapper lifetime, outer cancel/stop/suspend, HTTP method prefixes/wrap/raw queue leaks, bounded replacement-worker race, WWW cache thresholds/fallback, cancellation leaks/late resources, conversion failure, cookies, reentrant fanout and cache-control parsing including embedded NUL.

## Per-body C reconciliation

I = published portable algorithm/effect implemented on stated domain (not native glue or whole-owner equivalence). P = substantive partial body/snapshot/interior; exact missing boundary stated. U = unimplemented/unsupported. Documentation C status is never used as implementation credit. Ordinary Rust allocation failure is not converted into native false. Foreign HTTP completion/stop/drain evidence is implemented separately and adds zero WWW inventory bodies.

| Address | Bytes | Implementation disposition |
|---|---:|---|
| 00812de0 | 3607 | I: listing::parse (supported initialized/NUL/fixed-backing domains only) |
| 00814560 | 1878 | I: ftp::Session::transfer(false), transfer_body, progress, terminal |
| 00813e90 | 1741 | I: ftp::Session::transfer(true), transfer_body, progress, terminal |
| 00816ee0 | 1628 | P: worker::Scheduler prefix/final/transfer interior; fresh init 00813d70 and named lookup 0081e640 stay Unsupported; no native worker equivalence |
| 008163d0 | 1466 | P: entry/control/data loop and post-close suffix separately; O 00811f00/00811f90 barriers, unavailable borrowed-list ID Unsupported |
| 00815d30 | 1245 | P: login stages/boolean/state/notifications implemented for existing control/numeric address; O control creation/named lookup Unsupported |
| 008f7a50 | 1062 | I: service::cache_policy; six-token splitter, embedded-NUL C comparisons |
| 008f8ffe | 988 | P: cache/duplicates and header-free reached submission; cookie header record construction Unsupported; external transport absent |
| 008f8c5a | 932 | P: normal completion/fanout/conversion/cache/cookies; targetless matched suffix Unsupported Q10 |
| 00812390 | 825 | P: Session defaults used by algorithms; not a byte-layout/all auxiliary-field or native allocation implementation |
| 008f84ea | 483 | I: service::Www::local; foreign path/file primitives |
| 00818210 | 481 | P: Client logical constructor defaults incl nullable supplied lookup; native lookup/sentinel/thread allocation not implemented |
| 008f815f | 478 | I: service::Www::complete_ftp |
| 00816af0 | 451 | I: ftp::Session::disconnect; literal deferred operation 101 |
| 00812ba0 | 437 | I: ftp::Responses::read; malformed/short/full-line domains excluded |
| 00823000 | 432 | I: www::percent_decode |
| 008f833d | 429 | P: dispatch/target/handler/local/HTTP selection; FTP O admission and HTTP cookie support paths Unsupported |
| 00816230 | 415 | P: published replacement/name sequence implemented; native O node-allocation/cleanup helper equivalence not claimed |
| 008126d0 | 360 | I: listing::detect |
| 0081cda0 | 319 | P: wrapper ordering and temporary holds; PUT normalization O helpers and unsafe record-count/pointer domains Unsupported |
| 00817bd0 | 300 | I: http::method_id; strncmp length/prefix/NUL distinctions |
| 008f9bed | 299 | P: snapshot defaults only; full constructor Unsupported at P cookie constructor 008f9bad |
| 00816db0 | 296 | P: published shutdown/destruction effects; not native object-layout or allocator equivalence |
| 0081ca90 | 266 | P: header-free/non-normalizing admission and raw record copy; 0081acf0/0081ac20 Unsupported |
| 00816990 | 242 | P: published opcode decision, 101/102/111 and transfer state-4 rejection; O commands/live borrowed pointers Unsupported |
| 008f7966 | 234 | P: ordered directory/settings effects and cache result; foreign settings implementations excluded |
| 008f93da | 230 | U: FTP admission calls O 00817600; never supplied by opaque native-result callback |
| 008f736e | 227 | I: www::path_preference |
| 00811cd0 | 199 | I: ftp::classify |
| 00816cc0 | 197 | P: published initialized guard/target/socket/state effects; unrepresented auxiliary zero stores not claimed |
| 0081cc10 | 191 | I: http::Client::cancel; raw-node leaks and active detach retained |
| 00823310 | 190 | I: www::native_path |
| 008f7f04 | 182 | I: service::Www::cancel_all |
| 008120e0 | 174 | P: logical seven-argument message envelope and borrowed callback/ref order; native per-setter ABI calls not emulated |
| 008f8992 | 160 | P: shutdown-before-component/default releases; destructor subobject/layout allocation not emulated |
| 00812b10 | 144 | I: www::leaf; ordinary narrow Windows paths |
| 008f6c47 | 143 | I: service::Www::progress(ftp=true) |
| 008f7e76 | 142 | I: service::Www::cancel; no FTP cancel, no target release |
| 00811fc0 | 140 | I: listing::last_field |
| 008f6bbc | 139 | I: service::Www::progress(ftp=false); null unsafe domain excluded |
| 00811e70 | 138 | I: ftp::ensure_listener |
| 008f70c3 | 135 | I: service::Www::complete_http cookie-family loops, foreign cookie service |
| 00817580 | 123 | I: worker::Scheduler::start; active/hold not rolled back |
| 008f6a9a | 123 | I: service::Www::envelope terminal, deliver |
| 00812d60 | 123 | I: listing::next_line |
| 00812190 | 122 | I: worker::Scheduler::final_boundary borrowed job notification |
| 00813cb0 | 119 | I: worker::Job clone/value copy; no target ownership |
| 008f6b15 | 116 | I: service::Www::envelope progress |
| 00812290 | 110 | I: ftp::Session::send_command; original prefix resubmission, no zero-send repair |
| 00811da0 | 98 | I: ftp::send_chunks |
| 00813c50 | 94 | I: www::directory / service::Www::set_directory FTP effect |
| 00819650 | 94 | I: www::directory / service::Www::set_directory HTTP effect |
| 00812840 | 94 | I: worker::EmptyValue; unknown leading words remain Option::None |
| 008f698b | 90 | I: service::Www::do_message |
| 008f6e2b | 85 | I: service::Www::remove_handler |
| 00811f30 | 84 | I: ftp::ensure_data |
| 008f6d08 | 81 | I: service::Www::status (serial snapshot; lock effects at mutations) |
| 00817d00 | 80 | I: http::method_name |
| 008f687d | 73 | I: service::Www::query_interface; view offsets and retain effect |
| 008f72b5 | 72 | P: URL reuse and dispatch; same unsupported paths as URL overload |
| 00812970 | 71 | I: listing Vec node-sequence clearing; sentinel allocation not emulated |
| 008f692a | 65 | P: guarded cancel/handler/component shutdown sequence; foreign component internals excluded |
| 008f708a | 57 | I: service::Www::add_handler |
| 00813d30 | 53 | I: ftp::Responses::completed |
| 00818100 | 52 | I: http::Client::append; exact eight copied words |
| 008f6cd6 | 50 | I: service::Www::remove_all_handlers |
| 00816ac0 | 44 | I: ftp::Session::start; wrapper lifecycle, no session hold/join |
| 008f68fe | 44 | I: service::Www::post_init; ordered foreign component calls |
| 008f68c6 | 43 | U: AddRef numeric count/return body not implemented; QI/target retains are explicit effects |
| 008f7451 | 42 | I: Job Rust storage drop emits no target Release |
| 008196b0 | 42 | I: service::Www::set_directory HTTP cache-setting copy effect |
| 008f69e5 | 39 | I: service::Www::set_default_target |
| 008f6a0c | 36 | I: service::Www::http_id |
| 00817560 | 32 | I: worker::Scheduler::wrapper_return after worker exit |
| 008f6d59 | 31 | I: service::Www::get_job; active-only borrow |
| 00817540 | 30 | P: Session::destroy deleting-bit storage effect; native returned-this pointer not emulated |
| 00816210 | 30 | U: scheduler destructor 00813e10 is O; wrapper explicitly Unsupported |
| 008f6a30 | 29 | I: service::Www::ftp_id |
| 008f8c3e | 28 | U: individual destructor thunk/return ABI not specified/implemented |
| 00816d90 | 23 | P: Session shutdown/destruction interior; nullable native wrapper/virtual call not implemented |
| 00817790 | 22 | I: http::Client::release_request; unsigned >1 or delete |
| 008f6a4d | 19 | I: service::Www::visited; cache-use independent |
| 008f68f1 | 13 | I: service::Www::init |
| 008f6981 | 10 | I: service::Www::set_cache_usage |
| 008f6973 | 10 | I: service::Www::set_cookie_usage |
| 0097b497 | 10 | I: service::Www::set_default_custom |
| 008f696b | 8 | I: service::Www::get_directory |
| 00813bf7 | 6 | U: exception landing pad only; not independently callable Rust listing logic |
| 009d41a5 | 4 | U: native cookie-manager pointer getter not exposed by the portable service |
| 008f697d | 4 | I: service::Www::cache_usage |
| 0097e89e | 4 | I: service::Www::cookie_usage |

C attribution reconciliation: 63 I, 22 P, 6 U; 91 rows, 26,757 B of published contracts, not an implementation byte claim.

## Exact retained P/O boundary

All 57 original P/O rows / 7,292 B remain unsupported, without invented owner algorithms. Q1 cookie Port count and Q3 persistence remain unknown. Q7/Q9 retain their parked partial disposition; no fourth RE round requested.

| Address | Bytes | Original contract status |
|---|---:|---|
| 00815a60 | 717 | O: Q7 — unsupported |
| 00814cc0 | 657 | O: Q7 — unsupported |
| 00814f60 | 579 | O: Q7 — unsupported |
| 008153a0 | 579 | O: Q7 — unsupported |
| 008155f0 | 579 | O: Q7 — unsupported |
| 00815840 | 538 | O: Q7 — unsupported |
| 008151b0 | 489 | O: Q7 — unsupported |
| 00817600 | 399 | O: Q7 — unsupported |
| 00817af0 | 220 | O: Q7 — unsupported |
| 00813d70 | 154 | O: Q7 — unsupported |
| 008129c0 | 135 | O: Q7 — unsupported |
| 00812300 | 132 | O: Q7 — unsupported |
| 0081e640 | 128 | O: Q9 — unsupported |
| 0081da90 | 125 | O: Q9 — unsupported |
| 00813e10 | 120 | O: Q7 — unsupported |
| 00817a70 | 116 | O: Q7 — unsupported |
| 00812210 | 114 | O: Q7 — unsupported |
| 008f72fd | 113 | O: Q9 — unsupported |
| 008f80f2 | 109 | O: Q9 — unsupported |
| 00812aa0 | 104 | O: Q7 — unsupported |
| 00811e10 | 84 | O: Q7 — unsupported |
| 0081dd70 | 80 | O: Q9 — unsupported |
| 0081d8d0 | 79 | O: Q9 — unsupported |
| 0081dc60 | 78 | O: Q9 — unsupported |
| 00812a50 | 78 | O: Q7 — unsupported |
| 008f714a | 75 | O: Q9 — unsupported |
| 008f9bad | 64 | P: 3.6 / Q9 — unsupported |
| 008f747b | 49 | O: Q9 — unsupported |
| 008120b0 | 46 | O: Q7 — unsupported |
| 00812050 | 44 | O: Q7 — unsupported |
| 00817a40 | 39 | O: Q7 — unsupported |
| 00812080 | 37 | O: Q7 — unsupported |
| 008f86cd | 37 | O: Q9 — unsupported |
| 00811f90 | 33 | O: Q7 — unsupported |
| 008f7fd1 | 33 | O: Q9 — unsupported |
| 00811f00 | 33 | O: Q7 — unsupported |
| 0081ddc0 | 30 | O: Q9 — unsupported |
| 008f6d78 | 27 | O: Q9 — unsupported |
| 008f8a7a | 26 | O: Q9 — unsupported |
| 008f7fba | 23 | O: Q9 — unsupported |
| 0081d7d0 | 22 | O: Q9 — unsupported |
| 00811c90 | 21 | O: Q7 — unsupported |
| 008f7195 | 20 | O: Q9 — unsupported |
| 0081e770 | 19 | O: Q9 — unsupported |
| 0081e740 | 18 | O: Q9 — unsupported |
| 008f8a72 | 8 | O: Q9 — unsupported |
| 009cd02a | 8 | O: Q9 — unsupported |
| 008f8a6a | 8 | O: Q9 — unsupported |
| 008f8a42 | 8 | O: Q9 — unsupported |
| 008f8a3a | 8 | O: Q9 — unsupported |
| 008f8a32 | 8 | O: Q9 — unsupported |
| 008f8a4a | 8 | O: Q9 — unsupported |
| 008f8a5a | 8 | O: Q9 — unsupported |
| 008f8a52 | 8 | O: Q9 — unsupported |
| 008f8a62 | 8 | O: Q9 — unsupported |
| 0090ce99 | 5 | O: Q9 — unsupported |
| 0081e760 | 5 | O: Q9 — unsupported |

## Questions and next steps

Filed Q10 in docs/specs/questions/web-network.md: targetless matched HTTP completion suffix. No changes to generated QUESTIONS.md or tools; coordinator may regenerate the question index. Retain Q1/Q3/Q7/Q9. No speculative owner/native/default coverage claim. Next: finish queued checks, fix any diagnostics, run final stable full gate, update this report and queue only after successful validation.
