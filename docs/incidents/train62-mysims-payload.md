# Train62 shadow failure

The candidate replacement run reached2032/ref0. The shadow process exited in2029-06 after a null-vtable read in FC4MySimsShadow.cpp Query<cISC4Occupant>, reached through CanonicalOccupant and ShadowForwarder::DoMessage. The DLL/PDB and failed evidence are frozen under build-runs/run4-failures/train62-null-void1.

The forwarder queried every message void1 as a COM occupant before dispatch. Public mysims.md3.2 assigns that payload only to the three occupant notification families. The correction reads/converts it only for those families; a poison-callback regression test covers unrelated messages. No comparison counts, algorithms, sampling or goldens change.

The failing message type was not captured, so this does not prove which family triggered that run. A fresh full gate and unique-label full regression are required. The shadow source was unchanged between the preceding train and train62; no claim that either candidate introduced the latent decoder bug.
