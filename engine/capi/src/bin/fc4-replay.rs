// SPDX-License-Identifier: GPL-3.0-or-later
fn main() {
    let mut manifest = None;
    let mut no_cache = false;
    let mut jobs = std::thread::available_parallelism().map_or(1, usize::from);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--no-cache" => no_cache = true,
            "--jobs" => jobs = args.next().and_then(|s| s.parse().ok()).unwrap_or(0),
            "--manifest" => manifest = args.next(),
            _ => {
                eprintln!("usage: fc4-replay --manifest <manifest.tsv> [--no-cache] [--jobs N]");
                std::process::exit(2);
            }
        }
    }
    let Some(manifest) = manifest else {
        eprintln!("--manifest is required");
        std::process::exit(2);
    };
    match fc4_capi::trace::run(std::path::Path::new(&manifest), no_cache, jobs) {
        Ok(true) => (),
        Ok(false) => std::process::exit(1),
        Err(e) => {
            eprintln!("fc4-replay: {e}");
            std::process::exit(2);
        }
    }
}
