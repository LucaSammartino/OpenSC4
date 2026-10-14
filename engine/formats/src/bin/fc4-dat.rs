// SPDX-License-Identifier: GPL-3.0-or-later
//! `fc4-dat`: inspect SimCity 4 DBPF packages locally. Reads only; `extract` and `fsh … <out.tga>` write only the
//! output file named.

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use fc4_formats::exemplar::{ChainEnd, Exemplar, ExemplarStore, Kind, TextValues};
use fc4_formats::fsh::{self, Fsh};
use fc4_formats::s3d::{self, Model};
use fc4_formats::tgi::{TYPE_COHORT, TYPE_EXEMPLAR, TYPE_FSH, TYPE_LTEXT, TYPE_S3D};
use fc4_formats::{ltext, qfs, Package, Tgi};

const USAGE: &str = "\
usage:
  fc4-dat list <file>                              TGI, offset, size, compression of every entry
  fc4-dat exemplar <file> <T> <G> <I> [--resolve]  dump an exemplar or cohort (--resolve: add inherited
                                                   properties from parent cohorts in the same file)
  fc4-dat ltext <file> <T> <G> <I>                 print an LTEXT entry
  fc4-dat extract <file> <T> <G> <I> <out> [--raw] write an entry (decompressed unless --raw) to <out>
  fc4-dat fsh <file> <T> <G> <I> [<out.tga>]       FSH texture: header, entries, attachments, levels; with
                                                   <out.tga>, write frame 0 level 0 as a 32-bit TGA (local
                                                   inspection only: never commit decoded game data)
  fc4-dat s3d <file> <T> <G> <I>                   S3D model: version, chunks and their counts
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
        "fsh" => {
            let key = tgi(pos.get(2..).unwrap_or(&[]))?;
            fsh_cmd(&pkg, key, pos.get(5).map(String::as_str), o)
        }
        "s3d" => s3d_cmd(&pkg, tgi(pos.get(2..).unwrap_or(&[]))?, o),
        _ => Err(USAGE.into()),
    }
}

/// Prints an FSH record as the factory reads it (graphics-formats §2); with `tga`, writes frame 0 level 0 there.
fn fsh_cmd<S: fc4_formats::Source>(
    pkg: &Package<S>,
    key: Tgi,
    tga: Option<&str>,
    o: &mut dyn Write,
) -> Result<(), String> {
    let e = pkg.find(&key).ok_or_else(|| format!("no entry {key}"))?;
    let data = pkg.read(e).map_err(|e| e.to_string())?;
    let compressed = if pkg.is_compressed(e) {
        ", QFS-compressed in the package"
    } else {
        ""
    };
    say!(o, "FSH {key}: {} bytes{compressed}", data.len());
    if key.type_id != TYPE_FSH {
        eprintln!("note: type {:08X} is not the FSH type the factory creates", key.type_id);
    }
    let f = Fsh::parse(&data).map_err(|e| format!("not accepted: {e}"))?;
    let (format, ty) = f.format();
    say!(
        o,
        "size {}, {} entries, directory ID {:?}; texture {} x {}, code {:#04X}, M {} ({} levels), \
         format {} ({format:?}), type {} ({ty:?}), DX7 creates it: {}",
        f.data().len(),
        f.frames.len(),
        String::from_utf8_lossy(&f.directory_id),
        f.width,
        f.height,
        f.code(),
        f.mip_count(),
        f.levels.len(),
        format.number(),
        ty.number(),
        if fsh::dx7_creatable(format, ty) { "yes" } else { "no" }
    );
    for (k, l) in f.levels.iter().enumerate() {
        say!(
            o,
            "  level {k}: offset {}, size {}, {} x {}",
            l.offset,
            l.size,
            l.width,
            l.height
        );
    }
    for (i, (d, fr)) in f.directory.iter().zip(&f.frames).enumerate() {
        let h = fr.header;
        say!(
            o,
            "entry {i}: tag {:?}, offset {:#X}, code {:#04X}, {} x {}, A {}, M {}, centre ({}, {}), misc {:#010X}, \
             next {:#X}, data end {:#X}{}",
            String::from_utf8_lossy(&d.tag),
            d.offset,
            h.code,
            h.width,
            h.height,
            h.a(),
            h.mip_count(),
            h.centre_x,
            h.centre_y,
            h.misc,
            h.next,
            fr.data_end,
            if fr.zero_filled {
                ", unpadded mips: the aligned levels past the entry read the following record bytes (GF9)"
            } else {
                ""
            }
        );
        for a in f.attachments(i) {
            say!(
                o,
                "  attached block at {:#X}: code {:#04X}, next {:#X}",
                a.offset,
                a.code,
                a.next
            );
        }
        if let Some(p) = fr.palette {
            say!(
                o,
                "  palette at {:#X}: code {:#04X}, {} x {}",
                p.offset,
                p.code,
                p.width,
                p.height
            );
        } else if h.code == fsh::code::INDEX8 {
            say!(o, "  no usable palette: decodes as grey (FS4)");
        }
    }
    if let Some(path) = tga {
        let img = f.decode(0, 0).map_err(|e| e.to_string())?;
        std::fs::write(Path::new(path), tga_bytes(&img)).map_err(|e| format!("{path}: {e}"))?;
        say!(o, "wrote frame 0 level 0 ({} x {}) to {path}", img.width, img.height);
    }
    Ok(())
}

/// An uncompressed 32-bit TGA (rows top to bottom, 8 alpha bits) of an RGBA8 image.
fn tga_bytes(img: &fsh::Image) -> Vec<u8> {
    let mut out = vec![0u8; 18];
    out[2] = 2; // uncompressed true colour
    out[12..14].copy_from_slice(&(img.width as u16).to_le_bytes());
    out[14..16].copy_from_slice(&(img.height as u16).to_le_bytes());
    out[16] = 32;
    out[17] = 0x28; // 8 alpha bits, top-left origin
    for p in img.rgba.chunks_exact(4) {
        out.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
    }
    out
}

/// Prints an S3D model's chunks and counts (graphics-formats §3).
fn s3d_cmd<S: fc4_formats::Source>(pkg: &Package<S>, key: Tgi, o: &mut dyn Write) -> Result<(), String> {
    let e = pkg.find(&key).ok_or_else(|| format!("no entry {key}"))?;
    let data = pkg.read(e).map_err(|e| e.to_string())?;
    let compressed = if pkg.is_compressed(e) {
        ", QFS-compressed in the package"
    } else {
        ""
    };
    say!(o, "S3D {key}: {} bytes{compressed}", data.len());
    if key.type_id != TYPE_S3D {
        eprintln!("note: type {:08X} is not the S3D type", key.type_id);
    }
    let m = Model::parse(&data).map_err(|e| format!("does not load: {e}"))?;
    say!(o, "HEAD version {}.{}", m.version.major, m.version.minor);
    say!(o, "VERT {} blocks", m.vertex_blocks.len());
    for (i, v) in m.vertex_blocks.iter().enumerate() {
        say!(
            o,
            "  {i}: format {:#X} (packed {:#010X}), stride {} (drawn with {}), {} vertices, box {:?} to {:?}",
            v.format,
            v.packed_format().0,
            v.stride,
            v.draw_stride(),
            v.count,
            v.bbox.min,
            v.bbox.max
        );
    }
    say!(o, "INDX {} blocks", m.index_blocks.len());
    for (i, b) in m.index_blocks.iter().enumerate() {
        say!(o, "  {i}: {} indices", b.indices.len());
    }
    say!(o, "PRIM {} blocks", m.primitive_blocks.len());
    for (i, b) in m.primitive_blocks.iter().enumerate() {
        let list: Vec<String> = b
            .iter()
            .map(|p| format!("type {} first {} count {}", p.kind, p.first, p.count))
            .collect();
        say!(o, "  {i}: {}", list.join("; "));
    }
    say!(o, "MATS {} materials", m.materials.len());
    for (i, mat) in m.materials.iter().enumerate() {
        say!(
            o,
            "  {i}: flags {:#06X} (stored {:#X}), alpha {:?} ref {}/15, depth {:?}, blend ({:?}, {:?}), \
             {} of {} textures used",
            mat.flags,
            mat.stored_flags,
            s3d::CompareFunc::from_u8(mat.alpha_func),
            mat.alpha_ref_level(),
            s3d::CompareFunc::from_u8(mat.depth_func),
            s3d::BlendFactor::from_u8(mat.src_blend),
            s3d::BlendFactor::from_u8(mat.dst_blend),
            mat.texture_count,
            mat.textures.len()
        );
        for t in &mat.textures {
            say!(
                o,
                "     texture {:#010X}: wrap ({:?}, {:?}), filters ({:?}, {:?}), name {:?}",
                t.id,
                s3d::TextureParam::from_u8(t.wrap_s),
                s3d::TextureParam::from_u8(t.wrap_t),
                s3d::TextureParam::from_u8(t.mag_filter),
                s3d::TextureParam::from_u8(t.min_filter),
                t.name
            );
        }
    }
    let a = &m.anim;
    say!(
        o,
        "ANIM {} frames, rate {}, mode {}, flags {}, displacement {}; {} meshes kept, {} dropped",
        a.frame_count,
        a.frame_rate,
        a.mode,
        a.flags,
        a.displacement,
        a.meshes.len(),
        a.dropped
    );
    for mesh in &a.meshes {
        let first = mesh.frames.first().map(|b| format!("{b:?}")).unwrap_or_default();
        say!(
            o,
            "  {:?}: flags {}, frame 0 blocks (VERT, INDX, PRIM, MATS) {first}",
            mesh.name,
            mesh.flags
        );
    }
    if m.version.has_prop() {
        say!(o, "PROP {} entries", m.props.len());
        for p in &m.props {
            say!(o, "  mesh {} frame {}: {:?} = {:?}", p.mesh, p.frame, p.key, p.value);
        }
    }
    if m.version.has_regp() {
        say!(o, "REGP {} names", m.regions.len());
        for (name, list) in &m.regions {
            say!(o, "  {name:?}: {} transforms, first {:?}", list.len(), list.first());
        }
    }
    Ok(())
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
        ex.properties().len()
    );
    if key.type_id != TYPE_EXEMPLAR && key.type_id != TYPE_COHORT {
        eprintln!("note: type {:08X} is not the exemplar or cohort type", key.type_id);
    } else if let Err(e) = Exemplar::parse_record(key.type_id, &data) {
        eprintln!("note: the game's factory would reject this record: {e}");
    }
    if !resolve {
        for p in ex.properties() {
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
        ChainEnd::NotCohort(t) => say!(o, "  parent {t} is not a cohort; inherited values stop there"),
        ChainEnd::Cycle(t) => say!(o, "  parent chain loops back to {t}"),
        ChainEnd::TooDeep => say!(o, "  parent chain too deep"),
    }
    // The game's rule (data-formats §4.2): a parent that does not load fails the whole exemplar.
    match store.load(&key) {
        Ok(_) => say!(o, "  loads with this file alone"),
        Err(e) => say!(o, "  does not load with this file alone: {e}"),
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
