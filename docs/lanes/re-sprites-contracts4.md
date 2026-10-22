| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-sprites-contracts4: animated-sprite native contracts | `re-sprites-contracts4` (RE docs lane) | in progress; detached worktree reused; no runtime or coverage claims | Trace AS1/AS3–AS9 against native owner and reconcile specs/questions. |

## Handoff

- Goal: resolve only AS1/AS3–AS9 for cS3DModelInstanceAnimatedSprite; no engine reads/edits.
- Done: native-byte proof added for clock/reversal/returns, sequence selection/frame boundary quirks and fade/highlight/self-lit. AS8 disproves closed [0,1]: −100 no-op, −200 disable, <=0.99 accepts negatives/NaN, >0.99 disables.
- Next: trace load/resource/frame records and owner defaults/interfaces; reconcile answered questions after remaining proof.
- Native sources: C:/Users/tlori/FreeCity4/decomp/cS3DModelInstanceAnimatedSprite.c and tools/re_query.py in main checkout.
- q.py cannot start Bash (E_ACCESSDENIED); use local PowerShell commands with capped output.
- AS2 is outside scope and stays open. No game requests, ABI proposals or runtime validation.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-sprites-contracts4-r1-manifest.json.
