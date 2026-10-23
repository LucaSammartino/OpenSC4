# Portable city input owners

Behaviour source: city-tools.md sections 1, 2, 4 and quirks 1-3/7, with
city-tools-input.md section 2 and the named primitive service specs. This is a
new production family, separate from view_input_controls and player_drive.
No native tool, full base Init/COM/stack, TerrainScrubber, Query, PlaceLot,
Zone, Bulldozer, sound-manager consumer or C ABI is implemented here.

This inventory and fixtures.md precede source implementation. All entry
identifiers are published spec identifiers. Body ends/byte lengths are
unpublished; there is no byte credit or complete binary-owner claim.

## Method inventory

Supported means the published portable algorithm in the stated domain, not
the native entry point. Partial explicitly excludes the residuals below.

| Owner / published method | Portable API | Status / boundary |
|---|---|---|
| Flora Init 004BBA20 | Flora::init | Partial: id/cursor constants, supplied static price and explicitly acquired simulator token, zero total, initial price indicator; native base Init/static-data lookup/COM unsupported |
| Flora Shutdown 004BB940 | Flora::shutdown | Partial: release supplied acquired simulator then clear(true); native base shutdown/return unproved |
| Flora planting helper (unnamed) | internal plant | Supported: insertion(false), positioned instance sound, withdrawal, press/drag total asymmetry, indicator, aura; failed insertion water/result branch |
| Flora Press 004BBAF0 | Flora::press | Supported: terrain hit, funds gate, pressed on hit, true |
| Flora Move 004BBC80 | Flora::mouse_move | Supported: pressed affordability/plant, hover predicate, sticky shortage and every-move sound, true |
| Flora Release 004BB980 | Flora::release | Partial: pressed false, total-dependent indicator, reset total/latch, true; release clear's raw flag unpublished |
| Torch Init 00580660 | Torch::init | Partial: id/cursor, inactive, explicit held fire/timer tokens, start timer then on intent; base/native acquisition/return unsupported |
| Torch SendTorchToolAudioMessage 00580550 | primitive TorchAudio intent | Partial: owner chooses exact message and raw data; CT2 creation/post/consumer unsupported |
| Torch ActivateTorch 005806D0 | Torch::activate | Supported: effect/change gate, stop(0), release, create/start(0), optional release intent, active intent |
| Torch UpdateTorchEffect 00580760 | Torch::update_effect | Partial: retained position, identity rotation, scale 1; missing position explicitly unsupported |
| Torch ShutdownTorchEffect 005807F0 | Torch::drop_effect | Supported: started query, stop(1), release, inactive, active(false) intent |
| Torch OnMouseExit | Torch::mouse_exit | Partial: delegates effect drop; native return unpublished |
| Torch Move 00580A30 | Torch::mouse_move | Supported: pick/miss, lazy inactive effect, raw bit 0x10 activation, placement, last-active-move cell, move intent, true |
| Torch Press 005808B0 | Torch::press | Supported: press intent before pick, activate only existing effect, position, register(C0), immediate hit cell ignite(false,true,0), cause1, cursor, true |
| Torch animation message 86AD10EE / 00580510 | Torch::animation_tick | Supported: active gate, retained cell ignite(true,true,0) then cause1; no fabricated message return |
| Torch Release 005809D0 | Torch::release | Supported: release intent, unregister, deactivate, own cursor, true |
| Torch Shutdown 00580840 | Torch::shutdown | Partial: timer stop, effect drop, off intent, release held fire manager, unregister; no native return |
| CitySituations construct 004B8690 | CitySituations::new | Partial: identity constants; initial locked/pressed supplied explicitly (constructor field initialization unpublished) |
| CitySituations Init 004B84B0 | CitySituations::init | Partial: subscribe A6B79602 only; managers are primitive host services, native acquisition/lifetime/base unsupported |
| CitySituations Move 004B8620 | CitySituations::mouse_move | Partial: lock+state3 or unlock+forget (no immediate relock); empty hover with no lock unsupported; same handle leaves lock; false |
| CitySituations Press 004B8720 | CitySituations::press | Supported: level1 gate, indicator state2, remember, true; otherwise false |
| CitySituations Release 004B8790 | CitySituations::release | Partial: level1/both-managers gate, hit state1, initiate only matched+active occupant with null advice; literal hit returns true and retains pressed, refused/miss clears pressed; clarification filed |
| CitySituations DoMessage 004B85B0 | CitySituations::message | Partial: all three reset IDs unlock held handle, state(0,1), clear both, true; unknown message result unsupported |
| CitySituations Shutdown / native lifetime | none | Unsupported: no published method body contract |
| All native constructors/Init failures/QI/AddRef/Release/vtables/returns not explicitly published | none | Unsupported; tokens are portable identities, never pointers |

## Primitive and residual boundary

Hosts supply raw picks/world positions (including insertion's rewritten position),
price/funds, insertion/withdrawal/location/water results, sound instance IDs,
effect tokens/started state, timer counts, indicator lookup/occupants and manager
availability. They receive only primitive calls/intents. They never decide a
stroke, cell, effect transition, mission, mouse-handler result or indicator branch.
Existing Vec3/Pixel and cursor lookup/application are reused. Result sound is
the base slot11 primitive, not the existing different event-post helper.

Only published acquired/created references are explicitly released: supplied
held flora/fire tokens and created effects. Timer token is borrowed; no invented
timer Release. Indicator/occupant tokens are borrowed. No implicit Drop callbacks.
Only one owner may consume a held-resource obligation; shutdown consumes it.

Unsupported is typed, never a native failure/default: nonpositive price, integer
conversion/overflow, nonfinite coordinates, missing effect position, foreign
resource failure, use after shutdown, unspecified event data, empty-hover branch
and unknown message. Errors preserve their completed primitive prefix; callers
must not retry as if transactional. No clamp, wrapping, null handle, synthesized
timer count or effect fallback. Cost intents retain int64; raw slot42 narrowing
and release clear's flag are unclaimed. Missing constructor state is supplied,
except Torch's explicitly published fresh last-move cell (0,0).

No game request or native/shadow conformance is claimed for this portable module.
