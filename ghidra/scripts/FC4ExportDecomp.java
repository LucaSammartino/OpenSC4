// Decompile every function and write a per-namespace source tree.
// Usage (headless): -postScript FC4ExportDecomp.java <outDir>
// Output: <outDir>/<Namespace path>.c (functions in address order) and <outDir>/index.tsv
//@category FreeCity4
import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.concurrent.ConcurrentHashMap;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.decompiler.parallel.DecompileConfigurer;
import ghidra.app.decompiler.parallel.DecompilerCallback;
import ghidra.app.decompiler.parallel.ParallelDecompiler;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Program;
import ghidra.util.task.TaskMonitor;

public class FC4ExportDecomp extends GhidraScript {

    static String fileFor(Function f) {
        String ns = f.getParentNamespace().getName(true);
        if (f.getParentNamespace().isGlobal()) {
            String n = f.getName();
            if (n.startsWith("FUN_")) {
                // group unnamed code by 64 KiB region so files stay navigable
                long a = f.getEntryPoint().getOffset();
                return "_unnamed/region_" + String.format("%08x", a & ~0xFFFFL);
            }
            return "_global";
        }
        return ns.replace("::", "/").replaceAll("[<>:\"|?*]", "_");
    }

    @Override
    protected void run() throws Exception {
        File out = new File(getScriptArgs()[0]);
        out.mkdirs();
        Program p = currentProgram;
        List<Function> fns = new ArrayList<>();
        p.getFunctionManager().getFunctions(true).forEach(fns::add);
        println("decompiling " + fns.size() + " functions");
        Map<Long, String[]> results = new ConcurrentHashMap<>();
        DecompileConfigurer cfg = d -> {
            DecompileOptions o = new DecompileOptions();
            o.grabFromProgram(p);
            d.setOptions(o);
            d.toggleCCode(true);
            d.toggleSyntaxTree(false);
            d.setSimplificationStyle("decompile");
        };
        DecompilerCallback<Void> cb = new DecompilerCallback<>(p, cfg) {
            @Override
            public Void process(DecompileResults r, TaskMonitor m) {
                Function f = r.getFunction();
                String code = r.decompileCompleted() && r.getDecompiledFunction() != null
                    ? r.getDecompiledFunction().getC()
                    : "/* decompilation failed: " + r.getErrorMessage() + " */\n";
                results.put(f.getEntryPoint().getOffset(), new String[] { fileFor(f), code });
                return null;
            }
        };
        cb.setTimeout(120);
        ParallelDecompiler.decompileFunctions(cb, fns, monitor);
        cb.dispose();

        Map<String, TreeMap<Long, Function>> byFile = new TreeMap<>();
        for (Function f : fns) {
            byFile.computeIfAbsent(fileFor(f), k -> new TreeMap<>()).put(f.getEntryPoint().getOffset(), f);
        }
        try (PrintWriter idx = new PrintWriter(new FileWriter(new File(out, "index.tsv")))) {
            idx.println("address\tname\tfile\tbytes");
            for (Map.Entry<String, TreeMap<Long, Function>> e : byFile.entrySet()) {
                File file = new File(out, e.getKey() + ".c");
                file.getParentFile().mkdirs();
                try (PrintWriter w = new PrintWriter(new FileWriter(file))) {
                    w.println("// FreeCity4 decompilation of SimCity 4.exe 1.1.641 (Ghidra pseudocode, not original source)");
                    w.println("// Namespace: " + e.getKey());
                    w.println();
                    for (Function f : e.getValue().values()) {
                        String[] r = results.get(f.getEntryPoint().getOffset());
                        String cmt = f.getComment();
                        w.println("// ---- " + f.getEntryPoint() + "  " + f.getName(true));
                        if (cmt != null) for (String l : cmt.split("\n")) w.println("// " + l);
                        w.println(r == null ? "/* not decompiled */" : r[1]);
                        idx.println(f.getEntryPoint() + "\t" + f.getName(true) + "\t" + e.getKey() + ".c\t"
                            + f.getBody().getNumAddresses());
                    }
                }
            }
        }
        println("wrote " + byFile.size() + " files to " + out);
    }
}
