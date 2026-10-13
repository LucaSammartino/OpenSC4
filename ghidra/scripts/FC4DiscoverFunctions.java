// After auto-analysis, create functions at disassembled code that belongs to no function: an instruction that
// nothing falls into sequentially and that is either 16-byte aligned or referenced. Repeats until nothing changes.
// The game exe has many such bodies (virtual methods reached only through vtables, ICC-split code).
// Headless: -postScript FC4DiscoverFunctions.java
//@category FreeCity4
import ghidra.app.cmd.function.CreateFunctionCmd;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.*;

public class FC4DiscoverFunctions extends GhidraScript {

    private boolean fallsInto(Listing listing, Instruction ins) {
        Instruction prev = listing.getInstructionBefore(ins.getAddress());
        if (prev == null) return false;
        if (!prev.getMaxAddress().next().equals(ins.getAddress())) return false;
        Address ft = prev.getFallThrough();
        return ft != null && ft.equals(ins.getAddress());
    }

    @Override
    public void run() throws Exception {
        Listing listing = currentProgram.getListing();
        FunctionManager fm = currentProgram.getFunctionManager();
        int total = 0;
        for (int pass = 1; pass <= 10; pass++) {
            int created = 0;
            for (Instruction ins : listing.getInstructions(true)) {
                if (monitor.isCancelled()) return;
                Address a = ins.getAddress();
                if (fm.getFunctionContaining(a) != null) continue;
                if (fallsInto(listing, ins)) continue;
                boolean aligned = (a.getOffset() & 0xF) == 0;
                boolean referenced = currentProgram.getReferenceManager().hasReferencesTo(a);
                if (!aligned && !referenced) continue;
                CreateFunctionCmd cmd = new CreateFunctionCmd(a);
                if (cmd.applyTo(currentProgram, monitor)) created++;
            }
            println("FC4DiscoverFunctions pass " + pass + ": created " + created);
            total += created;
            if (created == 0) break;
        }
        println("FC4DiscoverFunctions created " + total + " functions; now " + fm.getFunctionCount());
    }
}
