// Attach Function ID databases before auto-analysis, so the Function ID analyzer names statically linked
// library functions (CRT, STLport, Lua, libpng, zlib).
// Headless: -preScript FC4AttachFid.java <fidb file or directory> ...
//@category FreeCity4
import java.io.File;

import ghidra.app.script.GhidraScript;
import ghidra.feature.fid.db.FidFile;
import ghidra.feature.fid.db.FidFileManager;

public class FC4AttachFid extends GhidraScript {
    @Override
    public void run() throws Exception {
        FidFileManager mgr = FidFileManager.getInstance();
        int n = 0;
        for (String arg : getScriptArgs()) {
            File f = new File(arg);
            File[] files = f.isDirectory() ? f.listFiles((d, name) -> name.endsWith(".fidb")) : new File[] { f };
            if (files == null) continue;
            for (File x : files) {
                FidFile ff = mgr.addUserFidFile(x);
                if (ff != null) { ff.setActive(true); n++; println("attached " + x); }
            }
        }
        println("FC4AttachFid attached " + n + " databases");
    }
}
