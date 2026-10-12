// SPDX-License-Identifier: GPL-3.0-or-later
//! `fc4-dat`: inspect SimCity 4 DBPF packages locally. Reads only; `extract` writes only the output file named.

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use fc4_formats::exemplar::{ChainEnd, Exemplar, ExemplarStore, Kind, TextValues};
use fc4_formats::tgi::{TYPE_COHORT, TYPE_EXEMPLAR, TYPE_LTEXT};
use fc4_formats::{ltext, qfs, Package, Tgi};

const USAGE: &str = "\
usage:
  fc4-dat list <file>                              TGI, offset, size, compression of every entry
  fc4-dat exemplar <file> <T> <G> <I> [--resolve]  dump an exemplar or cohort (--resolve: add inherited
                                                   properties from parent cohorts in the same file)
  fc4-dat ltext <file> <T> <G> <I>                 print an LTEXT entry
  fc4-dat extract <file> <T> <G> <I> <out> [--raw] write an entry (decompressed unless --raw) to <out>
IDs are hexadecimal, with or without 0x.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    match run(&args, &mut out).and_then(|()| out.flush().map_err(io_err)) {
        Ok(()) => ExitCode::SUCCESS,
        // The reader went away (for example `| head`): not an error.
        Err(e) if e.is_empty() => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fc4-dat: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Maps an output error to a message; a closed pipe becomes the empty message (see `main`).
fn io_err(e: std::io::Error) -> String {
    if e.kind() == std::io::ErrorKind::BrokenPipe {
        String::new()
    } else {
        format!("output: {e}")
    }
}

macro_rules! say {
    ($o:expr, $($t:tt)*) => {
        writeln!($o, $($t)*).map_err(io_err)?
    };
}

fn hex(s: &str) -> Result<u32, String> {
    let t = s.trim_start_matches("0x").trim_start_matches("0X");
    u32::from_str_radix(t, 16).map_err(|_| format!("not a hexadecimal ID: {s}"))
}

fn tgi(args: &[String]) -> Result<Tgi, String> {
    if args.len() < 3 {
        return Err(USAGE.into());
    }
    Ok(Tgi::new(hex(&args[0])?, hex(&args[1])?, hex(&args[2])?))
}

fn run(args: &[String], o: &mut dyn Write) -> Result<(), String> {
    let cmd = args.first().ok_or(USAGE)?;
    let file = args.get(1).ok_or(USAGE)?;
    let pkg = Package::open(file).map_err(|e| format!("{file}: {e}"))?;
    for w in pkg.warnings() {
        eprintln!("warning: {w}");
    }
    let flags: Vec<&str> = args
        .iter()
        .filter(|a| a.starts_with("--"))
        .map(String::as_str)
        .collect();
    let pos: Vec<String> = args.iter().filter(|a| !a.starts_with("--")).cloned().collect();
    match cmd.as_str() {
        "list" => list(&pkg, o),
        "exemplar" => exemplar(&pkg, tgi(pos.get(2..).unwrap_or(&[]))?, flags.contains(&"--resolve"), o),
        "ltext" => {
            let key = tgi(pos.get(2..).unwrap_or(&[]))?;
            if key.type_id != TYPE_LTEXT {
                eprintln!("note: type {:08X} is not the LTEXT type", key.type_id);
            }
            let data = pkg.read_tgi(&key).map_err(|e| e.to_string())?;
            say!(o, "{}", ltext::parse(&data).map_err(|e| e.to_string())?);
            Ok(())
        }
        "extract" => {
            let key = tgi(pos.get(2..).unwrap_or(&[]))?;
            let out = pos.get(5).ok_or(USAGE)?;
            let e = pkg.find(&key).ok_or_else(|| format!("no entry {key}"))?;
            let data = if flags.contains(&"--raw") {
                pkg.read_raw(e)
            } else {
                pkg.read(e)
            };
            let data = data.map_err(|e| e.to_string())?;
            std::fs::write(Path::new(out), &data).map_err(|e| format!("{out}: {e}"))?;
            say!(o, "wrote {} bytes to {out}", data.len());
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

fn list<S: fc4_formats::Source>(pkg: &Package<S>, o: &mut dyn Write) -> Result<(), String> {
    let h = pkg.header();
    say!(
        o,
        "DBPF {}.{}, index {}.{} ({}-byte entries), {} entries, {} DIR records",
        h.major,
        h.minor,
        h.index_major,
        h.index_minor,
        pkg.index_entry_size(),
        pkg.entries().len(),
        pkg.dir_entries().len()
    );
    say!(
        o,
        "{:<8} {:<8} {:<8} {:>10} {:>10} {:>10}  C",
        "type",
        "group",
        "instance",
        "offset",
        "size",
        "unpacked"
    );
    let mut compressed = 0;
    for e in pkg.entries() {
        let dir = pkg.dir_size(e);
        let mut flag = if dir.is_some() { "C" } else { "-" }.to_string();
        if dir.is_some() {
            compressed += 1;
            if let Ok(raw) = pkg.read_raw(e) {
                if !qfs::has_header(&raw) {
                    flag.push_str(" (listed, not QFS)");
                }
            }
        }
        let inst2 = e.instance2.map(|i| format!(" [{i:08X}]")).unwrap_or_default();
        say!(
            o,
            "{:08X} {:08X} {:08X}{inst2} {:>10} {:>10} {:>10}  {flag}",
            e.tgi.type_id,
            e.tgi.group,
            e.tgi.instance,
            e.offset,
            e.size,
            dir.map(|d| d.to_string()).unwrap_or_default()
        );
    }
    say!(o, "{} entries, {compressed} compressed", pkg.entries().len());
    Ok(())
}

fn exemplar<S: fc4_formats::Source>(
    pkg: &Package<S>,
    key: Tgi,
    resolve: bool,
    o: &mut dyn Write,
) -> Result<(), String> {
    let data = pkg.read_tgi(&key).map_err(|e| e.to_string())?;
    let ex = Exemplar::parse(&data).map_err(|e| e.to_string())?;
    let kind = if ex.kind == Kind::Exemplar {
        "exemplar"
    } else {
        "cohort"
    };
    say!(
        o,
        "{kind} {key} ({:?}), parent {}, {} properties",
        ex.encoding,
        ex.parent,
        ex.properties.len()
    );
    if key.type_id != TYPE_EXEMPLAR && key.type_id != TYPE_COHORT {
        eprintln!("note: type {:08X} is not the exemplar or cohort type", key.type_id);
    }
    if !resolve {
        for p in &ex.properties {
            print_prop(o, 0, p)?;
        }
        return Ok(());
    }
    let mut store = ExemplarStore::new();
    let rep = store.load_package(pkg);
    if !rep.failures.is_empty() {
        eprintln!("note: {} exemplars in the file did not parse", rep.failures.len());
    }
    let (chain, end) = store.chain(&ex);
    for (i, w) in chain.windows(2).enumerate() {
        say!(o, "  parent level {}: {}", i + 1, w[0].parent);
    }
    match end {
        ChainEnd::Root => say!(o, "  chain complete"),
        ChainEnd::Missing(t) => say!(o, "  parent {t} is not in this file; inherited values stop there"),
        ChainEnd::Cycle(t) => say!(o, "  parent chain loops back to {t}"),
        ChainEnd::TooDeep => say!(o, "  parent chain too deep"),
    }
    for (depth, p) in store.resolved_properties(&ex) {
        print_prop(o, depth, p)?;
    }
    Ok(())
}

fn print_prop(o: &mut dyn Write, depth: usize, p: &fc4_formats::Property) -> Result<(), String> {
    let from = if depth == 0 {
        String::new()
    } else {
        format!("  (inherited, level {depth})")
    };
    let shape = if p.is_array {
        format!("[{}]", p.len())
    } else {
        String::new()
    };
    say!(
        o,
        "  0x{:08X} {}{shape} = {{{}}}{from}",
        p.id,
        p.value_type().name(),
        TextValues(&p.values)
    );
    Ok(())
}
