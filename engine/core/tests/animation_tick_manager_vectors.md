# Independent expected ledger (before production outputs)

Source: animation-tick-manager.md §8. Twenty proposed vectors, no native execution.
Ordering tests use explicit u32 identities at low load, never host addresses.

| Vector | Independent expectation |
|---|---|
| V1 | Phases1,2,3,4,5,6,7,0; bucket lists02,01,03,01,02,01,0,01; totals8/4/2/1. |
| V2 | Priorities63/64/127/128/191/192 map3/2/2/1/1/0; FFFFFFFF Add uses0. |
| V3 | Duplicate Add retains0; cross-band Add retains1; bucket0 precedes1. |
| V4 | Sentinel removes3; absent-specific2 falls back to1;0 survives. |
| V5 | Priority change effects +temp,-old,+new,-temp; absent pointer gains membership. |
| V6 | Self Remove retains request; recursive tick0 without read/frame/drain; suppresses2; drain removes2 despite requested0. |
| V7 | Remove/Add FIFO ends present; Add/Remove ends absent; deferred target not delivered now. |
| V8 | Two queued duplicate Adds retain twice, suppress later membership, Find sees live; drain drops two temps. |
| V9 | Raw1999/2000/2001/499999/500000/500001 clamps2000/2000/2001/499999/500000/500000; save raw now. |
| V10 | FFFFFFF0→20 gives48→2000;10000→9999 givesFFFFFFFF→500000. |
| V11 | Bounds(0,0)/(7,3)/(9,9)/(80000000,1) become(1,1)/(7,7)/(9,9)/(80000000,80000000). |
| V12 | Step0/1999/2000/500000/500001 from last10000 gives data2 2000/2000/2000/500000/500000; raw last10000+step. |
| V13 | Armed phase1 last0=10000,last2=20000 gives shared110000 then120000; empty2 copies110000. |
| V14 | First Step pauses only, next arms; ticks skip/admit/skip; all read clock/increment frame. |
| V15 | Pause(false) at0 givesFFFFFFFF, IsPaused false but tick skips; Step restores0 without pending. |
| V16 | Auto99 skips;100 arms/Reset;0 then skips at interval100; repeat enable0 does not Restart, arms each tick. |
| V17 | Deltas2000,10000 return10000;9000,3000 return9000; empty admitted sweep returnsmin. |
| V18 | QI counts0→1→2; failed IID preserves sentinel; Release1 then0 destroys; fresh0 Release destroys. |
| V19 | External owner protects cycle break; message owns1 manager ref; Shutdown drops both A memberships. Pending cancellation is conceptual, withheld as safe public path. |
| V20 | Nearest/24-bit deltas2000/100000/500000 give40000000/42C80001/43FA0001. |

Extra branches: mutable message persists except next-bucket data2/3 rewrites;
changed minimum affects return selection; callback pause does not stop admitted
sweep; empty scheduled timestamps advance, unscheduled unchanged; phase0 rate
uses wrap difference+3 including zero divisor; wrapping frame/depth; destructor
does not release registrations/message. Unknown native resize and inherited
floating environments are conditional, never native goldens.
