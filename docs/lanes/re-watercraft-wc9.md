| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-wc9: watercraft behavior contracts | Codex / re-watercraft-wc9 (detached HEAD) | ready | Land completed WC5/WC9 and scoped WC3/WC4 answers; remaining model-only audit is WC10/WC11. |

## Report

Round 2 delivers reviewable spec text; it does not claim complete watercraft owner closure.

- Prior-round commits e3904bdd8 and 809ce4b06 answer WC9 ordered steering and WC5 grounding/contact/heel. Those answers remain included.
- 562a56ff9 adds §5.2.2 AI target production and direct publication, plus §5.1.6 common tween sampling. Turning toward the first midpoint produces an absolute turn target; turning toward the second preserves its sign. Translation includes water height, uses the supplied time without a clamp, and selects speed by the used allowance fraction. Tween ratio alone is explicitly rounded to float; nonpositive/NaN duration returns target, whereas unordered ratio survives.
- c862e2a84 adds §3.6 LoadModels control, result and partial-failure/retry behavior, and §5.1.7's explicit binding to graphics-formats §3.11 for shared effect-controller arithmetic/lifecycle. An already nonempty model vector returns false from LoadModels; a failed later acquisition leaves earlier models installed.
- Added 19 unique conformance vectors: four tween, eight AI target/publication, four model loading and three effect binding vectors. They are authored spec vectors, not executed game tests.
- WC3/WC4 original entries and progress provenance are archived with scoped DIFFERS answers dated 2026-10-24. Exactly scoped remaining work is in docs/specs/questions/watercraft.md: WC10 covers animated-resource acquisition, auxiliary loading and model positioning; WC11 covers slot-100 model publication and the positioning/heading-basis helpers. §10.2 carries the same boundary. No remaining AI-target, common-tween or shared effect-controller gap is asserted.
- Existing attribution remains 75 functions / 38,073 bytes, rather than historical 79 / 38,556. No new owner count was claimed.

Validation at substantive commit c862e2a84: doccheck check PASS; questions check PASS; git diff whitespace check PASS; the 19 new vector IDs are unique. Final report is documentation only. Cargo, clippy and CTest: n/a. Game runs: none; shadow: n/a. No pending game requests, ABI exports or proposed ABI rows.

Evidence: main-checkout re_query export/instructions with bytes, bounded decomp reads, the prior ignored PE constant/vtable helper, and the existing graphics-formats §3.11 numerical audit. No REA query or fallback this round. No engine/ files read or changed; no decompiled text, game assets or external files committed.

An initial archive edit failed because shell output used the wrong encoding; it made no changes. The corrected UTF-8 read succeeded. A subsequent whitespace check detected CRLF conversion in the archive; it was normalized and the repeated check passed. Git again emitted packed-refs.lock permission errors while successfully creating detached-HEAD commits. No shared refs/config/hooks or maintenance operations were changed.

No ABI-rows file is needed. The READY queue uses kind re, the reserved branch name and the full actual detached HEAD. Supervisor publication is required. Follow-up RE should start at WC11's 0x00508610 and 0x005094A0/0x0080B830, then WC10's resource and auxiliary helpers; these residuals do not hold the completed spec additions.
