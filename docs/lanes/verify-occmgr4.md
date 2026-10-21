| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-occmgr4 (verify the occupant-manager shadow, native original + occmgr_shadow=1) | `verify-occmgr4` / (engine lane) | ready (round 2): call-follow mode `occmgr_shadow_follow = 1`; every query month on the FOLLOWED core, 40084 boxes after real Inserts/Moves exact, resyncs 0. Gate: cargo 3944/0, clippy 0, ctest 64/64. Game: vom4-calls-2032a 72/72, vom4-calls-normal-a 86/86, vom4-msgmode-2032a 72/72 (round-1 mode unchanged), ref 0 | add `occmgr_shadow_follow = 1` to the regress shadows run; RemoveOccupants (slots 22/23), Move(post) and synchronous Move remain unexercised |

## Report (round 2; round 1 report is in git at 6a1b4d37)

**Round-1 gap.** Months in which an Insert/Remove message was still queued set occupants aside and ran their
query samples on a fresh core read from the record (only 49/72 and 158/188 months on the followed core), and Moves
were followed only when a Move message was posted (0 and 2 in the round-1 runs).

**Change** (`engine/plugin/FC4OccMgrShadow.cpp`, db66bd85; key in `FC4Config.cpp` kOtherKeys). New setting
`occmgr_shadow_follow` (default 0 = round-1 message mode, unchanged). With 1, the original's vtable slots 20-24
(Insert, Remove, RemoveOccupants(filter), RemoveOccupants(type), Move; §11, each entry checked against its 1.1.641
address or the probe's stub, chained in front of the probe so the probe still counts) forward to hooks that capture
the caller's typed arguments BEFORE the original runs (canonical occupant token, data4, shutdown, post, filter/type),
call the original, and AFTER it returns make the same call on the followed core under the shadow lock. The core
keeps its own host state; the original's stored box is never an input, only compared after a successful Insert or
Move (§5.1 steps 5-7); both return values are compared. Remove holds its own AddRef across the call (released after).
Messages are only recorded: the queued streams are compared by common prefix with tails carried to the next month
(`pending`); a Move delivered inside the original's Move (§5.4 step 9 synchronous) goes on its own stream. Selection
(set aside -> fresh core), sampling, probe and the month record comparison are untouched; SyncSilentMoves still runs.

**Game** (Konradshohe, all verdict clean, ref_warnings=0, foreign_exceptions=0, off-thread 0, stray 0):

| | vom4-calls-2032a (turbo to 2032) | vom4-calls-normal-a (300 s normal, script terrain x2 + fire) | vom4-msgmode-2032a (turbo, follow=0) |
|---|---|---|---|
| months / mismatches=0 | 72/72 | 86/86 | 72/72 |
| queries on followed core | 72 months, 6336/6336 | 86 months, 7568/7568 | 52 months 4576/4576 (+20 fresh 1760/1760) |
| set aside / resyncs | 0 / 0 | 0 / 0 | 183 / 0 |
| calls ins/rem/remall/move(post) | 10656/10711/0/15968(0) | 15776/15970/0/24308(0) | - |
| returns orig==core | 37335/37335 | 56054/56054 | - |
| boxes after Insert/Move | 26624/26624 | 40084/40084 | 11045/11045 |
| msgorder (queued) / sync moves | 21367/21367 / 0 | 31741/31741 / 0 (5 pending at the end) | 22220/22220 |
| silent moves / unfollowed / nested | 0/0/0 | 0/0/0 | 0 |

The ~40k Move calls are all `post = false` with unchanged cells, which round 1's month-level silent-move check could
not see; each is now compared box-by-box after the original's call. Script lines ran (SetAltitude 120..125 to 260,
60..63 to 270, auto fire started). Not exercised: RemoveOccupants slots 22/23, Move with post, synchronous Move,
nested calls. Thread probe saw no second thread, so OM1 concurrency stays unverified. No OM questions filed.

**Gate**: cargo test 3944/0, clippy 0, CMake build PASS, ctest 64/64.
