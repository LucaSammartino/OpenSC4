# video-preferences: answered questions (archive)
<!-- Archive for [video-preferences.md](../../video-preferences.md). Lanes do not read this file. Open questions are in ../video-preferences.md. -->

<!-- from: re-video-preferences4 (2026-10-23) -->
- ~~**VP1 - §6.1, remaining redraw CFG:** qualify every intervening branch of
  `SetButtonsBasedOnPreferences 0079020d–00791201`: raw fields, enum and
  checkbox inversions, first-redraw `W+1e` baseline captures, resolution
  selection thresholds, unsupported-control requests and exact reference order.
  Does the owner consult any mode/driver identity or merely fixed control IDs?
  Establish empty/absent/duplicate/equal/current-mode outcomes only if an owned
  branch actually admits those inputs. [only the first enum group, null-P path,
  final depth group and return cleanup are qualified; no native-mode fallback]~~
  **ANSWERED 2026-10-25 (re-videoprefs8):** video-preferences.md §9.2: the fourteen groups in order, their
  value tests, the inverted UI-translucency capture, the exact (width, height) pairs and the all-clear for any
  other pair (baseline 0). Only fixed control IDs are used; no mode list, driver or device is consulted.

<!-- from: re-video-preferences4 (2026-10-23) -->
- ~~**VP2 - §6.3–6.4, remaining events and resource semantics:** qualify all other
  IDs admitted by §5.1, including selection dependencies and the complete
  jump-table-driven radio groups. Establish literal resource labels/roles for
  `6a57da48`, `ea57da59`, `ea57da6f`, `ea5e99d9`, `2a57cb83/84` and actual
  `8810 +18/+1c` / widget `+110` primitive semantics without importing a
  foreign widget algorithm. Can a disabled native control generate the admitted
  event, and what owner branch then runs? [four action branches and the
  `ab81ded5–d7` group qualified; no UX-derived labels or disabled-event reachability]~~
  **ANSWERED 2026-10-25 (re-videoprefs8):** video-preferences.md §9.3 (every other ID is a radio click within
  its group or a no-op; no dependencies between groups), §9.4 (`+110` is SetFlag with mask 1, visibility),
  §9.6 (captions from the shipped UI script: Cancel, Accept, Default Settings, the restart notice panel and
  every row). The owner never tests the enabled state, so whatever event a control sends takes the same branch.

<!-- from: re-video-preferences4 (2026-10-23) -->
- ~~**VP3 - §2, §7.1, construction and initialization:** qualify the owner
  allocation/constructor caller, initial count and `W+10/12/14/16/18/1a/1c/1e`
  values, and middle `ShowDialog 00791677–00791d1d` binding sequence. Prove the
  complete raw-control list and null preconditions. Which owned instruction
  first sets `W+1e`, and which words remain original snapshots after an Auto
  request/redraw? [no candidate preference-object copy in the audited prefix;
  existing P is retained or borrowed from `00b43cd8`; full owner lifetime unknown]~~
  **ANSWERED 2026-10-25 (re-videoprefs8):** video-preferences.md §9.1 (constructor `00791360`: count and
  words 0, `W+1e` = 1; the region screen and 3D view callers allocate, hold, ShowDialog, DoModalWin, Release)
  and §9.4 (the 35 bindings, each `SetNotificationTarget(root)` without null tests). All baselines stay the
  opening snapshots after an Auto redraw.

<!-- from: re-video-preferences4 (2026-10-23) -->
- ~~**VP4 - §6.2–6.3, callsite-only App boundary:** establish declared calling
  convention/types and any caller-visible guarantees for A `+4c(P+f0c)` and
  `+50(P+f0c)` from existing authorized boundary evidence, plus independently
  supplied literal Auto output fixtures. The selected owner ignores return
  channels and calls SavePreferences before ApplyVideoPreferences. Does any
  other owned branch constrain the pointer's valid domain or notification
  sequencing? [do not reopen capped App/AP3–9, persistence, renderer or mode
  enumeration bodies; no literal defaults or default-then-close golden yet]~~
  **ANSWERED 2026-10-25 (re-videoprefs8):** video-preferences.md §9.5: cSC4App slots 19/20 (`0044a1c0`,
  `0044a330`), thiscall, one pointer, callee pops 4 (application.md §2, §13.3); no other owner branch uses the
  pointer. Auto output values belong to application.md, not this panel.

<!-- from: re-video-preferences4 (2026-10-23) -->
- ~~**VP5 - §7.2, modal primitive qualification:** provide the interface identity
  and declared types for service `(a417445e,5a4)`, its raw `+90/+94/+60` calls,
  and service `00b43ce0 +28/+18`, using interface/callsite evidence only. The
  owner retains R across the modal call, conditionally restores F on nonzero
  `+60(F)` AL and always returns true AL; does any supported foreign-failure
  domain exist before its unchecked service/manager dereferences? [normal
  valid-service sequence qualified; no graceful missing-service policy, no
  modal-loop implementation or global startup/catalogue timing closure]~~
  **ANSWERED 2026-10-25 (re-videoprefs8):** video-preferences.md §9.5: the window manager's GZGetFocus,
  SetFocus, IsWindowValid and DoModalWin (modal-dialogs.md §2.2) and the window-location service's restore and
  save, the same sequence as the audio and budget panels. No failure domain: every missing service is
  dereferenced.
