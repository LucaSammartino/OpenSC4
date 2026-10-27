# Dispatch vehicle view implementation questions

## DVV30-1 — Init packed render states (open)

`dispatch-vehicle-view.md` §§1.1, 5.6 identify three packed states at +0x14,
+0x1C and +0x24 and their use, but neither that spec nor dispatch.md §4.1.3
gives their packed values or construction inputs. What values does Init pack?
The portable view accepts adapter-supplied opaque state tokens in member order
and returns `Unsupported::OpenRenderStates` when these are absent. Drawing and
pass order are implemented independently of those missing packed words.

## DVV30-2 — Model spin row-2 product spill (open)

Section 5.7 step 5 says row 2's M[2][2] adds one x87 product and one float32
product, but does not identify which of x*s and z*c is spilled. Which one?
The core accepts a result when both interpretations give the same binary32
value; otherwise it returns `Unsupported::OpenSpinRow2` before writing M.
