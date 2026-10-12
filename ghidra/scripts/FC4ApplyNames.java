// Apply FreeCity4 ported names (re/names_windows.tsv) and SimCity4.gdt class types to the program.
// Usage (headless): -postScript FC4ApplyNames.java <names.tsv> <gdt>...
//@category FreeCity4
import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.Iterator;
import java.util.List;
import java.util.Map;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.data.CategoryPath;
import ghidra.program.model.data.DataType;
import ghidra.program.model.data.DataTypeConflictHandler;
import ghidra.program.model.data.DataTypeManager;
import ghidra.program.model.data.FileDataTypeManager;
import ghidra.program.model.data.Structure;
import ghidra.program.model.listing.CodeUnit;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.GhidraClass;
import ghidra.program.model.symbol.Namespace;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.SymbolTable;
import ghidra.app.util.NamespaceUtils;

public class FC4ApplyNames extends GhidraScript {

    /** Split "A<x::y>::B::method" on top-level "::" only. */
    static List<String> splitQualified(String s) {
        List<String> out = new ArrayList<>();
        int depth = 0, start = 0;
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            if (c == '<' || c == '(') depth++;
            else if (c == '>' || c == ')') depth--;
            else if (depth == 0 && c == ':' && i + 1 < s.length() && s.charAt(i + 1) == ':') {
                out.add(s.substring(start, i));
                start = i + 2;
                i++;
            }
        }
        out.add(s.substring(start));
        return out;
    }

    static String clean(String part) {
        String p = part.trim().replace(' ', '_');
        return p.isEmpty() ? "_" : p;
    }

    /** True if the 32-bit register (or a sub-register) is read on the straight-line entry path before
     *  being written. Stops at the first branch, call or return. */
    boolean readsBeforeWrite(Function fn, String reg) {
        ghidra.program.model.lang.Register r = currentProgram.getRegister(reg);
        ghidra.program.model.listing.Instruction ins = getInstructionAt(fn.getEntryPoint());
        for (int n = 0; ins != null && n < 40; n++) {
            for (Object o : ins.getInputObjects()) {
                if (o instanceof ghidra.program.model.lang.Register
                        && ((ghidra.program.model.lang.Register) o).getBaseRegister().equals(r.getBaseRegister())) {
                    // `xor ecx,ecx` style idioms read and write the same register: treat as a write
                    String m = ins.getMnemonicString().toUpperCase();
                    if ((m.equals("XOR") || m.equals("SUB")) && ins.getNumOperands() == 2
                            && ins.getDefaultOperandRepresentation(0).equals(ins.getDefaultOperandRepresentation(1))) {
                        return false;
                    }
                    if (m.equals("PUSH") || m.equals("CALL")) {
                        // pushing ECX is a save only if it's later popped; be conservative: a read
                    }
                    return true;
                }
            }
            for (Object o : ins.getResultObjects()) {
                if (o instanceof ghidra.program.model.lang.Register
                        && ((ghidra.program.model.lang.Register) o).getBaseRegister().equals(r.getBaseRegister())) {
                    return false;
                }
            }
            if (ins.getFlowType().isCall() || ins.getFlowType().isTerminal() || ins.getFlowType().isJump()) {
                return false;
            }
            ins = ins.getNext();
        }
        return false;
    }

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File tsv = new File(args[0]);
        DataTypeManager pdtm = currentProgram.getDataTypeManager();
        SymbolTable st = currentProgram.getSymbolTable();

        // Index class structures from the type archives by simple name.
        Map<String, DataType> archiveStructs = new HashMap<>();
        List<FileDataTypeManager> archives = new ArrayList<>();
        for (int i = 1; i < args.length; i++) {
            FileDataTypeManager fdtm = FileDataTypeManager.openFileArchive(new File(args[i]), false);
            archives.add(fdtm);
            Iterator<DataType> it = fdtm.getAllDataTypes();
            while (it.hasNext()) {
                DataType dt = it.next();
                if (dt instanceof Structure && !archiveStructs.containsKey(dt.getName())) {
                    archiveStructs.put(dt.getName(), dt);
                }
            }
            println("archive " + args[i] + ": " + archiveStructs.size() + " structures indexed so far");
        }

        int renamed = 0, missing = 0, failed = 0, typed = 0;
        Map<String, Boolean> classTyped = new HashMap<>();
        try (BufferedReader br = new BufferedReader(new FileReader(tsv))) {
            String line = br.readLine(); // header
            while ((line = br.readLine()) != null) {
                if (monitor.isCancelled()) break;
                String[] f = line.split("\t", -1);
                if (f.length < 4) continue;
                Address addr = toAddr(Long.parseLong(f[0], 16));
                Function fn = getFunctionAt(addr);
                if (fn == null && f[2].startsWith("manual") && getInstructionAt(addr) == null) {
                    disassemble(addr);
                }
                if (fn == null && f[2].startsWith("manual") && getInstructionAt(addr) != null
                        && getFunctionContaining(addr) == null) {
                    fn = createFunction(addr, null); // hand-verified entry the deep pass missed
                }
                if (fn == null) { missing++; continue; }
                String full = f[1];
                String note = "";
                int paren = full.indexOf(" (");
                if (paren > 0) { note = full.substring(paren + 1); full = full.substring(0, paren); }
                try {
                    List<String> parts = splitQualified(full);
                    Namespace ns = currentProgram.getGlobalNamespace();
                    for (int i = 0; i < parts.size() - 1; i++) {
                        ns = NamespaceUtils.createNamespaceHierarchy(clean(parts.get(i)), ns, currentProgram,
                            SourceType.ANALYSIS);
                    }
                    String leaf = clean(parts.get(parts.size() - 1));
                    boolean isClassMethod = parts.size() >= 2 && !full.startsWith("lib::")
                        && !full.startsWith("icf::") && !full.startsWith("community::") && !full.startsWith("std::")
                        && !full.startsWith("__gnu_cxx::");
                    if (isClassMethod && !(ns instanceof GhidraClass)) {
                        ns = NamespaceUtils.convertNamespaceToClass(ns);
                    }
                    fn.setParentNamespace(ns);
                    fn.setName(leaf, SourceType.IMPORTED);
                    renamed++;
                    // Mac/GCC class layouts in SimCity4.gdt do NOT match the Windows/ICC layout, so we never
                    // type `this` with them. Instead, methods that Ghidra already sees taking ECX (and not
                    // EDX) become __thiscall; Ghidra then gives `this` a Windows-native placeholder struct.
                    if (isClassMethod && ns instanceof GhidraClass && !fn.hasCustomVariableStorage()
                            && readsBeforeWrite(fn, "ECX") && !readsBeforeWrite(fn, "EDX")) {
                        fn.setCallingConvention("__thiscall");
                        typed++;
                    }
                    StringBuilder c = new StringBuilder();
                    c.append("FreeCity4: ").append(f[1]).append("\nsource=").append(f[2])
                        .append(" confidence=").append(f[3]);
                    if (f.length > 4 && !f[4].isEmpty()) c.append("\n").append(f[4]);
                    if (!note.isEmpty()) c.append("\nnote: ").append(note);
                    fn.setComment(c.toString());
                }
                catch (Exception e) {
                    failed++;
                    if (failed < 30) println("failed " + f[0] + " " + full + ": " + e.getMessage());
                }
            }
        }
        for (FileDataTypeManager a : archives) a.close();
        println("renamed=" + renamed + " missing=" + missing + " failed=" + failed + " thiscallConverted=" + typed);
    }
}
