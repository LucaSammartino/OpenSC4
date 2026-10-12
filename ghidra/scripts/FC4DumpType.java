// Print every data type whose name matches, with category, size and components.
// Usage (headless): -postScript FC4DumpType.java <name>
//@category FreeCity4
import java.util.ArrayList;
import java.util.List;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.data.DataType;
import ghidra.program.model.data.DataTypeComponent;
import ghidra.program.model.data.Structure;

public class FC4DumpType extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String name = getScriptArgs()[0];
        List<DataType> found = new ArrayList<>();
        currentProgram.getDataTypeManager().findDataTypes(name, found);
        for (DataType dt : found) {
            println("TYPE " + dt.getPathName() + " size=" + dt.getLength() + " class=" + dt.getClass().getSimpleName());
            if (dt instanceof Structure) {
                int n = 0;
                for (DataTypeComponent c : ((Structure) dt).getDefinedComponents()) {
                    if (n++ > 25) { println("  ..."); break; }
                    println(String.format("  +0x%03x %-24s %s", c.getOffset(), c.getFieldName(), c.getDataType().getName()));
                }
            }
        }
    }
}
