# Independent LocalGridHelper literals

Transcribed from public local-grid-helper §§5.1–5.2 before production implementation/output comparison.
All native vectors remain PROPOSED / UNEXECUTED. Portable tests use raw primitive replies.

| ID | x,y,z | s1,s2,s3,s4,s5 | Qx,Fx; Qz,Fz | Tx,Ty,Tz,scale words | flags |
|---|---|---|---|---|---|
| G1 | 17.5,-3.25,-0.5 | 16,16,16,16,16 | 35/32,1; -1/32,-1 | 41880000 C0500000 C1700000 43800000 | 3 |
| G2 | 17.5,-3.25,-0.5 | 8,8,8,8,8 | 35/16,2; -1/16,-1 | 41840000 C0500000 C0F00000 43000000 | 3 |
| G3 | 17.5,-3.25,-0.5 | 8,32,4,64,2 | 35/16,2; -1/8,-1 | 41900000 C0500000 00000000 42000000 | 3 |
| G4 | 3/32,-3.25,-1/32 | 1/16,1/16,1/16,1/16,1/16 | 3/2,1; -1/2,-1 | 3D880000 C0500000 BD700000 3F800000 | 1 |
| G5 | +0,-0,+0 | 16,16,16,16,16 | +0,+0; +0,+0 | 3F800000 80000000 3F800000 43800000 | 3 |
| G6 | 17.5,-3.25,-0.5 | 16,16,16,16,3/32 | 35/32,1; -1/32,-1 | 41880000 C0500000 C1700000 3FC00000 | 3 |
| G7 | 17.5,-3.25,-0.5 | 16,16,16,16,1/16+2^-30 | 35/32,1; -1/32,-1 | 41880000 C0500000 C1700000 3F800000 | 3 |

G7 held product = 1+2^-26; stored scale = 1. This is an explicit ST(0) input, not a binary32 service declaration.

Boundary family: y=-3.25,z=-0.5, all five replies 16; Qz=-1/32,Fz=-1;
Ty=C0500000,Tz=C1700000,scale=43800000,flags=3 for every row.

| B | x word | Qx | Fx | Tx word |
|---|---|---|---|---|
| 1 | C1800000 | -1 | -1 | C1700000 |
| 2 | BE800000 | -1/64 | -1 | C1700000 |
| 3 | 80000000 | -0 | -0 | 3F800000 |
| 4 | 00000000 | +0 | +0 | 3F800000 |
| 5 | 417C0000 | 63/64 | 0 | 3F800000 |
| 6 | 417FFFFF | 1-2^-24 | 0 | 3F800000 |
| 7 | 41800000 | 1 | 1 | 41880000 |

## Lifecycle ledger

C=name request with direct writable H; Q=active query; S=Start; K=Stop; R=Release.
All S/K arguments are full u32 zero. E and F are distinct opaque identities.

| ID | Inputs | Expected |
|---|---|---|
| L1 | construct; C writes E, any status | initial null/no calls; C(local_grid,&H), H=E |
| L2 | repeat Init with E | no calls, same E |
| L3 | null; two Init, C untouched/failure | one C each, H=null |
| L4 | null; C writes E/failure; Init again | one C total, H=E |
| L5 | null; show 1,show 0,move,shutdown,destroy | no reads/calls |
| L6 | show 1/Q0,show 1/Q1 | Q,S0,Q |
| L7 | show 0/Q1,show 0/Q0 | Q,K0,Q |
| L8 | show 1/Q0 twice | Q,S0,Q,S0 |
| L9 | show 00000100/Q1,show 00000002/Q0 | Q,K0,Q,S0 |
| L10 | hide/Q0; G1,G1,x=17.75,x=32 | Q; eight move events each, first three G1, fourth Tx=42040000 |
| L11 | show 1/Q0; G1 | Q,S0; eight G1 events |
| L12 | shutdown/Q1; shutdown; destroy | Q,K0,clear H,R(E); no later calls |
| L13 | shutdown/Q0; Init/C writes F | Q,clear H,R(E),C; H=F |
| L14 | destroy live E | R(E) only; retained E observed at entry, lifetime consumed |
