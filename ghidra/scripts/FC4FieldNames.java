// Name recovered Windows struct fields from re/fields_windows.tsv
// (class, offset, name, type, confidence, source). A field that already exists at that offset is
// renamed; an undefined gap is filled with the given type. Recovered fields of a different size are
// never overwritten. Template classes apply to every instantiation (e.g. cSC4SimGrid<...>).
// Usage (headless): -postScript FC4FieldNames.java <fields.tsv>
//@category FreeCity4
import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.data.BooleanDataType;
import ghidra.program.model.data.ByteDataType;
import ghidra.program.model.data.DataType;
import ghidra.program.model.data.DataTypeComponent;
import ghidra.program.model.data.DataTypeManager;
import ghidra.program.model.data.FloatDataType;
import ghidra.program.model.data.IntegerDataType;
import ghidra.program.model.data.PointerDataType;
import ghidra.program.model.data.Structure;
import ghidra.program.model.data.UnsignedIntegerDataType;

public class FC4FieldNames extends GhidraScript {

    DataType typeFor(String t) {
        switch (t) {
            case "int": return IntegerDataType.dataType;
            case "uint": return UnsignedIntegerDataType.dataType;
            case "bool": return BooleanDataType.dataType;
            case "byte": return ByteDataType.dataType;
            case "float": return FloatDataType.dataType;
            case "ptr": return PointerDataType.dataType;
            default: return null; // embedded object / unknown: rename only
        }
    }

    static boolean rangeUndefined(Structure s, int off, int len) {
        for (int i = off; i < off + len; i++) {
            DataTypeComponent c = s.getComponentAt(i);
            if (c != null && c.getDataType() != null && !c.getDataType().getName().startsWith("undefined")) {
                return false;
            }
        }
        return true;
    }

    @Override
    protected void run() throws Exception {
        DataTypeManager dtm = currentProgram.getDataTypeManager();
        List<Structure> structs = new ArrayList<>();
        Iterator<Structure> it = dtm.getAllStructures();
        while (it.hasNext()) structs.add(it.next());
        int renamed = 0, filled = 0, skipped = 0;
        try (BufferedReader br = new BufferedReader(new FileReader(new File(getScriptArgs()[0])))) {
            br.readLine();
            String line;
            while ((line = br.readLine()) != null) {
                String[] f = line.split("\t", -1);
                if (f.length < 4) continue;
                String cls = f[0].trim();
                int off = Integer.decode(f[1].trim());
                String name = f[2].trim();
                String comment = "FreeCity4 (" + (f.length > 4 ? f[4] : "") + "): " + (f.length > 5 ? f[5] : "");
                for (Structure s : structs) {
                    if (!s.getCategoryPath().isRoot()) continue;
                    String sn = s.getName();
                    if (!(sn.equals(cls) || sn.startsWith(cls + "<"))) continue;
                    if (off >= s.getLength()) { skipped++; continue; }
                    DataTypeComponent c = s.getComponentAt(off);
                    try {
                        if (c != null && c.getOffset() == off && c.getDataType() != null
                                && !c.getDataType().getName().startsWith("undefined")) {
                            c.setFieldName(name);
                            c.setComment(comment);
                            renamed++;
                        }
                        else {
                            DataType dt = typeFor(f[3].trim());
                            if (c != null && c.getOffset() == off && dt == null) {
                                c.setFieldName(name);
                                c.setComment(comment);
                                renamed++;
                            }
                            else if (dt != null && off + dt.getLength() <= s.getLength()
                                    && rangeUndefined(s, off, dt.getLength())) {
                                s.replaceAtOffset(off, dt, dt.getLength(), name, comment);
                                filled++;
                            }
                            else skipped++;
                        }
                    }
                    catch (Exception e) {
                        skipped++;
                        println("skip " + sn + "+" + f[1] + " " + name + ": " + e.getMessage());
                    }
                }
            }
        }
        println("fieldsRenamed=" + renamed + " filled=" + filled + " skipped=" + skipped);
    }
}
