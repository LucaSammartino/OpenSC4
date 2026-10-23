# Literal expectations recorded before source

These are independent prose-derived oracles, not captured native outputs. Trace
values below are literals; setup uses primitive stubs. No native tool call or
host-supplied handler/transition result is allowed. P=(33.75,4.9,65.25), type=7,
price=100, sound instance=9. Hit cell=(2,4); sound position=(33,4,65).

## Flora (city-tools 1; quirks 1/2; flora 6.5)

F1. Init with held simulator=11: price-indicator(100), total=0. Press hit P,
funds50: pick, funds only, pressed=true, total=0, true. Release:
indicator(100,100), pressed=false, total=0, shortage=false, true.
F2. Following F1's press, two moves with hit/funds50 each produce pick, funds,
result(AA62C9D5); shortage=true. A subsequent miss still plays result. A funded
move doesn't clear the latch; result remains last. Release clears it.
F3. Funds100, insertion success, withdraw=false: press traces insertion(7,P,false),
new-instance9, event(EA451903,49DC0C90,9), event(CA45190F,9,(33,4,65)),
withdraw100, aura(33.75,65.25,1); total remains0, no total indicator. Drag with
the same primitive outcomes adds100 and indicator(100,100) before aura despite
failed withdrawal. Release positive total emits clear(positive-stroke), resets.
F4. Withdraw=true press: total100 and indicator before aura. Insertion failure:
water-test at (2,4); water=true => result; dry => no result; no sound instance,
withdrawal, total change or aura. Negative P=(-31.5,4.9,-47.75) => water vertex
(-1,-2), explicitly truncation toward zero, not floor.
F5. Hover hit calls location predicate(7,P.x,P.z,None) and saves its bool;
miss retains previous validity. Shortage sounds after all hover work too.
F6. Insertion may rewrite P to Q=(48.9,6.8,80.7): successful sound uses
(48,6,80) and aura uses(48.9,80.7). Failed insertion water-test uses rewritten Q.
F7. Price 3_000_000_000 remains int64 in totals and indicator intent. Arithmetic
overflow reports typed residual at addition after sound+withdraw prefix, no
wrapped indicator/aura. Nonpositive price and nonfinite conversion unsupported.
F8. Shutdown releases only held simulator11, then clear(true); repeat/use after
shutdown typed unsupported. No base/native return guessed.

## Torch (city-tools 2; quirk 3)

T1. Init starts timer then on(2A106113). No fabricated timer count. Fresh cell
(0,0), no position/effect. Press misses: press(8A10611D,x,y,count), pick, true;
no registration/fire/cursor. Press hit without effect: place-position stored,
register(C0), ignite(hit,false,true,0), cause1, cursor0A08C54E; still inactive,
tick does nothing. Press does not manufacture an effect or last-move cell.
T2. Move hit P, modifiers0: create(torch_inactive)=21, start(21,0), transform
(P,identity,1), move(4A106126,x,y,count), inactive/cell(0,0).
Press hit Q=(112,8,144): press intent, pick, stop(21,0), release21,
create(torch_active)=22,start(22,0), active(AA96FE8F,true), transform(Q),
register(C0), ignite((7,9),false,true,0), cause1, active cursor.
Next tick ignites((0,0),true,true,0), cause1: press-hit != retained-move cell.
T3. Active move P: transform(P), cell=(2,4), move intent. Release: release
intent, unregister, stop(active,0), release, create(inactive), start(0), release
intent again, active(false), own cursor4A08C57E. Next press Q fires(7,9),
next active tick fires retained(2,4). Cell survives release and drop.
T4. Fresh move with raw modifiers0x01 or0x08 remains inactive; 0x10 (or0x90)
activates via replacement, then transform and stores cell. Later move0 does
not deactivate. Same active flag => no replacement/messages.
T5. Move miss: drop checks started; if true stop(effect,1), release, inactive,
active(false), then move intent. If not started: release without stop. Exit
does same drop but no move intent. Drop without effect still sends active(false).
T6. Shutdown order: stop timer, drop(stop1/release/activefalse), off(8A10611A),
release held fire manager, unregister. Registration removed even without effect.
T7. Foreign create failure after stop0/release leaves a typed residual and no
fallback effect; no invented native success/failure. Audio on/off raw payload
not published: payload explicitly unresolved, never zeros. Counts come from
timer primitive only for press/release/move and deactivation release.

## CitySituations (city-tools 4; quirk 7)

C1. Explicit initial locked=None,pressed=None. Init subscribes A6B79602 only.
Move over7: find7, lock(7,true), state(7,3), locked7, false. Move7 again:
find7 only, false. Move8: find8, lock(7,false), locked=None, false; next move8
locks8 and sets state3. Miss while locked unlocks; empty miss is residual.
C2. Viewlevel2 press: level only, false; release: level only, clears pressed,
false. At level1 press7: level, find7, state(7,2), pressed7, true.
C3. At level1 with both managers, release7 after press7: level, availability,
find7, state(7,1), active-occupant=31, initiate(31,None), true. Initiate's bool
does not become handler result. Missing occupant: same prefix, no initiate.
Release8: state(8,1), no occupant lookup or initiate. Literal reading retains
pressed and returns true for each indicator hit; clarification filed.
C4. Release miss or missing manager: forget pressed, false; no state/initiate.
C5. For EACH 06CDB65B,A6B79602,A6B79612 with locked7/pressed8:
lock(7,false), state(0,1), locked=None,pressed=None, true. Reset without a lock:
state(0,1) only. Unknown message => typed unsupported, no effects.
C6. No fake CitySituations shutdown/refcount/COM result or pointer is tested.

Tests must assert these literal ordered primitive traces plus visible owner
state, including counterexamples to press/drag equivalence, press/tick cell
equivalence, immediate hover relock and subscription-only message handling.
