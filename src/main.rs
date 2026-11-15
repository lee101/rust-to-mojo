use rust_to_mojo::{translate_path, write_output, Options};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
rust-to-mojo: translate a Rust file or crate into Mojo, with a coverage report.

USAGE:
    rust-to-mojo <INPUT> [-o <OUT_DIR>] [--optimize-hints] [--no-runtime] [--quiet] [--top N]

ARGS:
    <INPUT>              a .rs file, or a crate directory (Cargo.toml / src/lib.rs)

OPTIONS:
    -o, --out <DIR>      output directory (default: out)
        --optimize-hints annotate hot loops / numeric code and write hints.json
        --no-runtime     do not copy the mojo_rt shim into the output directory
    -q, --quiet          only print the one-line summary
        --top <N>        number of ranked reasons to print (default 10)
    -h, --help           print this help
    -V, --version        print the version
";

struct Cli {
    input: PathBuf,
    out: PathBuf,
    optimize_hints: bool,
    no_runtime: bool,
    quiet: bool,
    top: usize,
}

fn parse_args() -> Result<Cli, String> {
    let mut args = std::env::args().skip(1);
    let mut input = None;
    let mut cli = Cli { input: PathBuf::new(), out: PathBuf::from("out"), optimize_hints: false, no_runtime: false, quiet: false, top: 10 };
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{}", USAGE);
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("rust-to-mojo {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-o" | "--out" => cli.out = PathBuf::from(args.next().ok_or("missing value for --out")?),
            "--optimize-hints" => cli.optimize_hints = true,
            "--no-runtime" => cli.no_runtime = true,
            "-q" | "--quiet" => cli.quiet = true,
            "--top" => cli.top = args.next().ok_or("missing value for --top")?.parse().map_err(|_| "--top expects a number")?,
            s if s.starts_with('-') => return Err(format!("unknown option `{}`", s)),
            s => {
                if input.is_some() {
                    return Err(format!("unexpected extra argument `{}`", s));
                }
                input = Some(PathBuf::from(s));
            }
        }
    }
    cli.input = input.ok_or("missing <INPUT>")?;
    Ok(cli)
}

fn main() -> ExitCode {
    let cli = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}\n\n{}", e, USAGE);
            return ExitCode::from(2);
        }
    };
    // Translation panics are caught per item and reported; keep stderr clean.
    std::panic::set_hook(Box::new(|_| {}));
    let opts = Options { optimize_hints: cli.optimize_hints };
    let out = match translate_path(&cli.input, &opts) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(2);
        }
    };
    if let Err(e) = write_output(&out, &cli.out, !cli.no_runtime, &opts) {
        eprintln!("error writing output: {}", e);
        return ExitCode::from(2);
    }
    let r = &out.report;
    let c = &r.counts;
    println!(
        "{}: {} files, {} items: {} translated, {} partial, {} stubbed, {} skipped -> emitted {:.1}%, clean {:.1}%{}",
        r.crate_name,
        r.files.len(),
        c.total,
        c.translated,
        c.partial,
        c.stubbed,
        c.skipped,
        r.emitted_pct,
        r.clean_pct,
        if r.internal_errors > 0 { format!(" ({} internal errors)", r.internal_errors) } else { String::new() }
    );
    if !cli.quiet {
        println!("\ntop stub reasons:");
        for (i, s) in r.stub_reasons.iter().take(cli.top).enumerate() {
            println!("  {:>2}. {:>5}  {}", i + 1, s.count, s.key);
        }
        if !r.partial_reasons.is_empty() {
            println!("\ntop TODO notes on partial items:");
            for (i, s) in r.partial_reasons.iter().take(cli.top).enumerate() {
                println!("  {:>2}. {:>5}  {}", i + 1, s.count, s.key);
            }
        }
        println!("\nwrote {} (coverage.md, coverage.json{})", cli.out.display(), if cli.optimize_hints { ", hints.json" } else { "" });
    }
    ExitCode::SUCCESS
}
