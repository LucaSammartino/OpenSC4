// Export the analysis tables that tools/symport reads: functions, function details, calls, string references,
// symbols, data references and decoded instructions. Works on any program (the Windows exe or the Mac reference).
// Headless: -postScript FC4ExportTables.java <output-directory>
//@category FreeCity4
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.*;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;

public class FC4ExportTables extends GhidraScript {

    private static String hex(Address a) {
        if (a.isExternalAddress()) return "EXTERNAL:" + String.format("%08x", a.getOffset());
        return String.format("%08x", a.getOffset());
    }

    private static String clean(String s) {
        if (s == null) return "";
        StringBuilder b = new StringBuilder(s.length());
        for (char c : s.toCharArray()) {
            if (c == '\t') b.append("\\t");
            else if (c == '\n') b.append("\\n");
            else if (c == '\r') b.append("\\r");
            else if (c < 0x20) b.append(String.format("\\x%02x", (int) c));
            else b.append(c);
        }
        return b.toString();
    }

    private PrintWriter open(File dir, String name, String header) throws IOException {
        PrintWriter w = new PrintWriter(new BufferedWriter(new OutputStreamWriter(
            new FileOutputStream(new File(dir, name)), StandardCharsets.UTF_8), 1 << 20));
        if (header != null) w.println(header);
        return w;
    }

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 1) throw new IllegalArgumentException("usage: FC4ExportTables.java <output-directory>");
        File out = new File(args[0]);
        out.mkdirs();
        Listing listing = currentProgram.getListing();
        ReferenceManager refs = currentProgram.getReferenceManager();
        FunctionManager fm = currentProgram.getFunctionManager();

        int nf = 0;
        try (PrintWriter funcs = open(out, "functions.tsv", "address\tname\tbody_bytes\tthunk\tstatus\tfile\tnotes\tsource");
             PrintWriter det = open(out, "function-details.tsv", "address\tname\tprototype\tcalling_convention\tranges");
             PrintWriter calls = open(out, "calls.tsv", "caller\tcallsite\ttarget\ttype");
             PrintWriter strs = open(out, "string-references.tsv", "function\treference_from\tstring_address\tvalue")) {
            for (Function f : fm.getFunctions(true)) {
                if (monitor.isCancelled()) break;
                String entry = hex(f.getEntryPoint());
                String name = clean(f.getName(true));
                AddressSetView body = f.getBody();
                funcs.println(entry + "\t" + name + "\t" + body.getNumAddresses() + "\t" + f.isThunk()
                    + "\texported\t\t\t" + f.getSymbol().getSource());
                StringBuilder ranges = new StringBuilder();
                for (AddressRange r : body) ranges.append(hex(r.getMinAddress())).append('-').append(hex(r.getMaxAddress())).append(';');
                det.println(entry + "\t" + name + "\t" + clean(f.getPrototypeString(false, false)) + "\t"
                    + clean(f.getCallingConventionName()) + "\t" + ranges);
                for (Instruction ins : listing.getInstructions(body, true)) {
                    for (Reference r : ins.getReferencesFrom()) {
                        RefType t = r.getReferenceType();
                        Address to = r.getToAddress();
                        if (t.isCall()) {
                            calls.println(entry + "\t" + hex(ins.getAddress()) + "\t" + hex(to) + "\t" + t.getName());
                        } else if (to.isMemoryAddress()) {
                            Data d = listing.getDataAt(to);
                            if (d != null && d.hasStringValue()) {
                                Object v = d.getValue();
                                strs.println(entry + "\t" + hex(ins.getAddress()) + "\t" + hex(to) + "\t"
                                    + clean(v == null ? "" : v.toString()));
                            }
                        }
                    }
                }
                nf++;
            }
        }

        int ns = 0;
        try (PrintWriter syms = open(out, "symbols.tsv", "address\ttype\tsource\tname")) {
            for (Symbol s : currentProgram.getSymbolTable().getAllSymbols(true)) {
                if (monitor.isCancelled()) break;
                syms.println(hex(s.getAddress()) + "\t" + s.getSymbolType() + "\t" + s.getSource() + "\t" + clean(s.getName(true)));
                ns++;
            }
        }

        int nd = 0;
        try (PrintWriter dref = open(out, "data-references.tsv", "from\tto\ttype")) {
            AddressIterator it = refs.getReferenceSourceIterator(currentProgram.getMemory(), true);
            while (it.hasNext() && !monitor.isCancelled()) {
                Address from = it.next();
                if (listing.getInstructionAt(from) != null) continue; // instruction operands are in calls/strings
                for (Reference r : refs.getReferencesFrom(from)) {
                    if (r.getReferenceType().isData() && r.getToAddress().isMemoryAddress()) {
                        dref.println(hex(from) + "\t" + hex(r.getToAddress()) + "\t" + r.getReferenceType().getName());
                        nd++;
                    }
                }
            }
        }

        long ni = 0;
        try (PrintWriter asm = open(out, "instructions.asm",
                "; Ghidra decoded instructions: virtual address, bytes, assembly. Not an assemblable reconstruction.")) {
            for (Instruction ins : listing.getInstructions(true)) {
                if (monitor.isCancelled()) break;
                StringBuilder b = new StringBuilder();
                for (byte x : ins.getBytes()) b.append(String.format("%02x", x & 0xff));
                asm.println(hex(ins.getAddress()) + "\t" + b + "\t" + ins);
                ni++;
            }
        }

        try (PrintWriter sum = open(out, "export-summary.txt", null)) {
            sum.println("program=" + currentProgram.getName());
            sum.println("executable_sha256=" + currentProgram.getExecutableSHA256());
            sum.println("image_base=" + currentProgram.getImageBase());
            sum.println("functions=" + nf);
            sum.println("symbols=" + ns);
            sum.println("data_references=" + nd);
            sum.println("instructions=" + ni);
        }
        println("FC4ExportTables wrote " + nf + " functions, " + ns + " symbols, " + nd + " data refs, " + ni
            + " instructions to " + out);
    }
}
