// Apply FreeCity4 data labels (re/labels_windows.tsv): vtables become `Class::vftable[_N]` labels
// typed as pointer arrays; other entries become plain labels with a comment.
// Usage (headless): -postScript FC4ApplyLabels.java <labels.tsv>
//@category FreeCity4
import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;

import ghidra.app.script.GhidraScript;
import ghidra.app.util.NamespaceUtils;
import ghidra.program.model.address.Address;
import ghidra.program.model.data.ArrayDataType;
import ghidra.program.model.data.PointerDataType;
import ghidra.program.model.listing.CodeUnit;
import ghidra.program.model.symbol.Namespace;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.util.CodeUnitInsertionException;
import ghidra.program.model.data.DataUtilities;

public class FC4ApplyLabels extends GhidraScript {
    @Override
    protected void run() throws Exception {
        File tsv = new File(getScriptArgs()[0]);
        int ok = 0, typed = 0, failed = 0;
        try (BufferedReader br = new BufferedReader(new FileReader(tsv))) {
            br.readLine();
            String line;
            while ((line = br.readLine()) != null) {
                String[] f = line.split("\t", -1);
                if (f.length < 3) continue;
                Address a = toAddr(Long.parseLong(f[0], 16));
                if (getFunctionAt(a) != null) continue; // named by FC4ApplyNames
                String full = f[1];
                int slots = f[2].isEmpty() ? 0 : Integer.parseInt(f[2]);
                try {
                    java.util.List<String> parts = FC4ApplyNames.splitQualified(full);
                    Namespace ns = currentProgram.getGlobalNamespace();
                    for (int i = 0; i < parts.size() - 1; i++) {
                        ns = NamespaceUtils.createNamespaceHierarchy(FC4ApplyNames.clean(parts.get(i)), ns,
                            currentProgram, SourceType.ANALYSIS);
                    }
                    Symbol s = currentProgram.getSymbolTable().createLabel(a,
                        FC4ApplyNames.clean(parts.get(parts.size() - 1)), ns, SourceType.IMPORTED);
                    s.setPrimary();
                    ok++;
                    if (slots > 0) {
                        try {
                            DataUtilities.createData(currentProgram, a,
                                new ArrayDataType(new PointerDataType(), slots, 4), -1, false,
                                DataUtilities.ClearDataMode.CLEAR_ALL_CONFLICT_DATA);
                            typed++;
                        }
                        catch (CodeUnitInsertionException e) {
                            // leave existing data in place
                        }
                    }
                    if (f.length > 3 && !f[3].isEmpty()) {
                        currentProgram.getListing().setComment(a, CodeUnit.PLATE_COMMENT, "FreeCity4: " + f[3]);
                    }
                }
                catch (Exception e) {
                    failed++;
                    if (failed < 20) println("failed " + f[0] + " " + full + ": " + e.getMessage());
                }
            }
        }
        println("labels=" + ok + " vtablesTyped=" + typed + " failed=" + failed);
    }
}
