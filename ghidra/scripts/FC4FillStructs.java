// Grow each class's Windows-native `this` struct from field accesses across all of its __thiscall
// methods (Ghidra FillOutStructure, merged per class). Mac/GCC layouts are never used.
// Usage (headless): -postScript FC4FillStructs.java [classNameFilter]
//@category FreeCity4
import java.util.ArrayList;
import java.util.List;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.decompiler.util.FillOutStructureHelper;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.data.DataType;
import ghidra.program.model.data.Pointer;
import ghidra.program.model.data.Structure;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.GhidraClass;
import ghidra.program.model.pcode.HighFunction;
import ghidra.program.model.pcode.HighSymbol;
import ghidra.program.model.pcode.HighVariable;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolType;

public class FC4FillStructs extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String filter = getScriptArgs().length > 0 ? getScriptArgs()[0] : null;
        DecompInterface d = new DecompInterface();
        DecompileOptions o = new DecompileOptions();
        o.grabFromProgram(currentProgram);
        d.setOptions(o);
        d.toggleCCode(false);
        d.openProgram(currentProgram);
        int classes = 0, methods = 0, grown = 0;
        List<GhidraClass> all = new ArrayList<>();
        currentProgram.getSymbolTable().getClassNamespaces().forEachRemaining(all::add);
        for (GhidraClass c : all) {
            if (monitor.isCancelled()) break;
            if (filter != null && !c.getName(true).equals(filter)) continue;
            List<Function> fns = new ArrayList<>();
            for (Symbol s : currentProgram.getSymbolTable().getSymbols(c)) {
                if (s.getSymbolType() == SymbolType.FUNCTION) {
                    Function f = getFunctionAt(s.getAddress());
                    if (f != null && "__thiscall".equals(f.getCallingConventionName())
                            && !f.getName().startsWith("~") && f.getParameterCount() > 0) {
                        fns.add(f);
                    }
                }
            }
            if (fns.isEmpty()) continue;
            classes++;
            for (Function f : fns) {
              try {
                DecompileResults r = d.decompileFunction(f, 60, monitor);
                HighFunction hf = r.getHighFunction();
                if (hf == null) continue;
                if (hf.getLocalSymbolMap().getNumParams() == 0) continue;
                HighSymbol ts = hf.getLocalSymbolMap().getParamSymbol(0);
                if (ts == null) continue;
                HighVariable hv = ts.getHighVariable();
                if (hv == null) continue;
                DataType dt = hv.getDataType();
                if (!(dt instanceof Pointer)) continue;
                Structure before = FillOutStructureHelper.getStructureForExtending(dt);
                if (before == null) continue;
                int sizeBefore = before.getLength();
                int compsBefore = before.getNumDefinedComponents();
                FillOutStructureHelper h = new FillOutStructureHelper(currentProgram, monitor);
                Structure st = h.processStructure(hv, f, false, true, d);
                methods++;
                if (methods <= 5 || (filter != null && methods <= 50)) {
                    println("  " + f.getName() + ": before=" + sizeBefore + " result="
                        + (st == null ? "null" : st.getPathName() + " size=" + st.getLength()
                            + " comps=" + st.getNumDefinedComponents())
                        + " loads=" + h.getLoadPcodeOps().size() + " stores=" + h.getStorePcodeOps().size());
                }
                if (st != null && (st.getLength() > sizeBefore || st.getNumDefinedComponents() > compsBefore)) {
                    // processStructure returns an unresolved copy; write it back into the program's type
                    currentProgram.getDataTypeManager().resolve(st,
                        ghidra.program.model.data.DataTypeConflictHandler.REPLACE_HANDLER);
                    grown++;
                }
              }
              catch (Exception e) {
                println("skip " + f.getName(true) + ": " + e);
              }
            }
            if (classes % 100 == 0) println("classes " + classes + " methods " + methods + " grown " + grown);
        }
        d.dispose();
        println("classes=" + classes + " methods=" + methods + " grownUpdates=" + grown);
    }
}
