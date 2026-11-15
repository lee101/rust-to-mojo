//! File and crate drivers: module-tree discovery, symbol collection, output
//! layout, runtime shim copy and report writing.

use crate::hints::Hint;
use crate::ir::MImport;
use crate::lower::items::ChildMod;
use crate::lower::{FileCtx, Options};
use crate::printer::print_module;
use crate::report::{CrateReport, FileReport, Reason};
use crate::symbols::Symbols;
use std::path::{Path, PathBuf};

/// One Rust module to translate.
struct Unit {
    module_path: Vec<String>,
    /// Source file the items come from.
    file: PathBuf,
    /// Display name for reports (`src/foo.rs` or `src/foo.rs#mod bar`).
    label: String,
    src: String,
    items: Vec<syn::Item>,
    inner_attrs: Vec<syn::Attribute>,
    parse_error: Option<String>,
    /// Directory where this module's file-backed children live.
    child_dir: PathBuf,
    has_children: bool,
    notes: Vec<Reason>,
}

pub struct Output {
    /// (relative output path, contents)
    pub files: Vec<(PathBuf, String)>,
    pub report: CrateReport,
    pub hints: Vec<Hint>,
}

pub const RUNTIME_FILES: &[(&str, &str)] = &[
    ("mojo_rt/__init__.mojo", include_str!("../mojo_rt/__init__.mojo")),
    ("mojo_rt/num.mojo", include_str!("../mojo_rt/num.mojo")),
    ("mojo_rt/option.mojo", include_str!("../mojo_rt/option.mojo")),
    ("mojo_rt/vec.mojo", include_str!("../mojo_rt/vec.mojo")),
    ("mojo_rt/dict.mojo", include_str!("../mojo_rt/dict.mojo")),
    ("mojo_rt/cell.mojo", include_str!("../mojo_rt/cell.mojo")),
    ("mojo_rt/range.mojo", include_str!("../mojo_rt/range.mojo")),
];

fn parse(src: &str) -> Result<syn::File, String> {
    syn::parse_file(src).map_err(|e| {
        let lc = e.span().start();
        format!("{} (line {}, col {})", e, lc.line, lc.column)
    })
}

fn sanitize_pkg(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true) {
        format!("_{}", s)
    } else {
        s
    }
}

/// Find crate name and root source file for a crate directory.
fn crate_root(dir: &Path) -> Result<(String, PathBuf), String> {
    let manifest = dir.join("Cargo.toml");
    let mut name = dir.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "crate".into());
    let mut lib_path: Option<PathBuf> = None;
    if let Ok(text) = std::fs::read_to_string(&manifest) {
        if let Ok(v) = text.parse::<toml::Table>() {
            if let Some(n) = v.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
                name = n.to_string();
            }
            if let Some(lib) = v.get("lib") {
                if let Some(n) = lib.get("name").and_then(|n| n.as_str()) {
                    name = n.to_string();
                }
                if let Some(p) = lib.get("path").and_then(|n| n.as_str()) {
                    lib_path = Some(dir.join(p));
                }
            }
        }
    }
    let root = lib_path
        .or_else(|| [dir.join("src/lib.rs"), dir.join("src/main.rs"), dir.join("lib.rs"), dir.join("main.rs")].into_iter().find(|p| p.exists()))
        .ok_or_else(|| format!("no crate root found under {} (looked for Cargo.toml [lib] path, src/lib.rs, src/main.rs)", dir.display()))?;
    Ok((sanitize_pkg(&name), root))
}

fn load_unit(file: &Path, module_path: Vec<String>, child_dir: PathBuf, base: &Path) -> Unit {
    let label = file.strip_prefix(base).unwrap_or(file).to_string_lossy().replace('\\', "/");
    let src = std::fs::read_to_string(file).unwrap_or_default();
    match parse(&src) {
        Ok(f) => Unit { module_path, file: file.to_path_buf(), label, src, items: f.items, inner_attrs: f.attrs, parse_error: None, child_dir, has_children: false, notes: vec![] },
        Err(e) => Unit { module_path, file: file.to_path_buf(), label, src, items: vec![], inner_attrs: vec![], parse_error: Some(e), child_dir, has_children: false, notes: vec![] },
    }
}

/// Discover all modules reachable from `root` via `mod` declarations.
fn discover(root: &Path, base: &Path) -> Vec<Unit> {
    let root_dir = root.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut units = vec![load_unit(root, vec![], root_dir, base)];
    let mut i = 0;
    while i < units.len() {
        let mut new_units = vec![];
        let mut notes = vec![];
        let items = units[i].items.clone();
        let mp = units[i].module_path.clone();
        let child_dir = units[i].child_dir.clone();
        for item in &items {
            let syn::Item::Mod(m) = item else { continue };
            if crate::lower::is_test_only(&m.attrs) {
                continue;
            }
            let name = m.ident.to_string();
            let mut sub = mp.clone();
            sub.push(name.clone());
            match &m.content {
                Some((_, inner)) => {
                    let u = Unit {
                        module_path: sub,
                        file: units[i].file.clone(),
                        label: format!("{}#mod {}", units[i].label, name),
                        src: units[i].src.clone(),
                        items: inner.clone(),
                        inner_attrs: m.attrs.iter().filter(|a| a.path().is_ident("doc")).cloned().collect(),
                        parse_error: None,
                        child_dir: child_dir.join(&name),
                        has_children: false,
                        notes: vec![],
                    };
                    new_units.push(u);
                }
                None => {
                    let path_attr = m.attrs.iter().find(|a| a.path().is_ident("path")).and_then(|a| {
                        if let syn::Meta::NameValue(nv) = &a.meta {
                            if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value {
                                return Some(s.value());
                            }
                        }
                        None
                    });
                    let candidates = match &path_attr {
                        Some(p) => vec![child_dir.join(p), units[i].file.parent().unwrap_or(Path::new(".")).join(p)],
                        None => vec![child_dir.join(format!("{}.rs", name)), child_dir.join(&name).join("mod.rs")],
                    };
                    match candidates.into_iter().find(|p| p.exists()) {
                        Some(p) => {
                            let cd = if p.file_name().map(|f| f == "mod.rs").unwrap_or(false) { p.parent().unwrap().to_path_buf() } else { child_dir.join(&name) };
                            let mut u = load_unit(&p, sub, cd, base);
                            if let Some(c) = crate::lower::cfg_of(&m.attrs) {
                                u.notes.push(Reason { key: "cfg-gated module translated unconditionally".into(), detail: format!("cfg({})", c) });
                            }
                            new_units.push(u);
                        }
                        None => notes.push(Reason { key: "module file not found".into(), detail: format!("mod {} (from {})", name, units[i].label) }),
                    }
                }
            }
        }
        if !new_units.is_empty() {
            units[i].has_children = true;
        }
        units[i].notes.extend(notes);
        units.extend(new_units);
        i += 1;
    }
    units
}

fn out_path(pkg: &str, unit: &Unit, single_file: bool) -> PathBuf {
    let mut p = PathBuf::new();
    if single_file && unit.module_path.is_empty() && !unit.has_children {
        p.push(format!("{}.mojo", pkg));
        return p;
    }
    p.push(pkg);
    for s in &unit.module_path {
        p.push(crate::lower::names::ident(s));
    }
    if unit.has_children || unit.module_path.is_empty() {
        p.push("__init__.mojo");
    } else {
        p.set_extension("mojo");
    }
    p
}

pub fn translate_path(input: &Path, opts: &Options) -> Result<Output, String> {
    let (pkg, root, base, single_file) = if input.is_dir() {
        let (pkg, root) = crate_root(input)?;
        (pkg, root, input.to_path_buf(), false)
    } else if input.is_file() {
        let stem = input.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "module".into());
        (sanitize_pkg(&stem), input.to_path_buf(), input.parent().unwrap_or(Path::new(".")).to_path_buf(), true)
    } else {
        return Err(format!("input not found: {}", input.display()));
    };
    let units = discover(&root, &base);

    let mut syms = Symbols { crate_name: pkg.clone(), ..Default::default() };
    for u in &units {
        syms.modules.insert(u.module_path.join("::"));
        syms.collect_items(&u.items, &u.module_path);
    }

    let mut files = vec![];
    let mut reports = vec![];
    let mut hints = vec![];
    let mut internal = 0;
    for u in &units {
        let op = out_path(&pkg, u, single_file);
        let mut fr = FileReport {
            source: u.label.clone(),
            output: op.to_string_lossy().replace('\\', "/"),
            module: if u.module_path.is_empty() { pkg.clone() } else { format!("{}.{}", pkg, u.module_path.join(".")) },
            ..Default::default()
        };
        fr.file_notes.extend(u.notes.iter().cloned());
        if let Some(e) = &u.parse_error {
            fr.parse_error = Some(e.clone());
            fr.file_notes.push(Reason { key: "syn parse error".into(), detail: e.clone() });
            let text = format!(
                "# TODO(rust-to-mojo): file `{}` could not be parsed by syn: {}\n# Nothing was translated.\n",
                u.label, e
            );
            files.push((op, text));
            reports.push(fr);
            continue;
        }
        let mut ctx = FileCtx::new(&syms, &u.src, opts, u.module_path.clone(), pkg.clone());
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ctx.lower_module(&u.inner_attrs, &u.items)));
        match res {
            Ok(mut out) => {
                internal += out.internal_errors;
                // `from . import child` is only valid inside a package __init__.
                if !u.has_children && !u.module_path.is_empty() || single_file && !u.has_children {
                    out.module.imports.retain(|i: &MImport| i.module != ".");
                }
                if !out.module.items.is_empty() || out.module.doc.is_some() || !out.module.imports.is_empty() {
                    out.module.header_comments.insert(0, format!("Generated by rust-to-mojo {} from {} -- do not edit by hand; see coverage.md", env!("CARGO_PKG_VERSION"), u.label));
                }
                let text = print_module(&out.module);
                for r in out.records {
                    fr.push(r);
                }
                fr.file_notes.extend(out.file_notes);
                for h in std::mem::take(&mut ctx.hints) {
                    let parts: Vec<&str> = h.splitn(3, '\t').collect();
                    if parts.len() == 3 {
                        hints.push(Hint { file: u.label.clone(), item: parts[0].into(), kind: parts[1].into(), message: parts[2].into() });
                    }
                }
                files.push((op, text));
            }
            Err(_) => {
                internal += 1;
                fr.file_notes.push(Reason { key: "internal error (translator panic at module level)".into(), detail: u.label.clone() });
                files.push((op, format!("# TODO(rust-to-mojo): internal translator error on `{}`\n", u.label)));
            }
        }
        reports.push(fr);
    }
    let report = CrateReport::build(&pkg, reports, internal);
    Ok(Output { files, report, hints })
}

/// Translate a single source string (used by golden tests and the library API).
pub fn translate_source(src: &str, module_name: &str, opts: &Options) -> (String, FileReport) {
    let mut fr = FileReport { source: module_name.into(), module: module_name.into(), ..Default::default() };
    let file = match parse(src) {
        Ok(f) => f,
        Err(e) => {
            fr.parse_error = Some(e.clone());
            return (format!("# TODO(rust-to-mojo): could not parse: {}\n", e), fr);
        }
    };
    let mut syms = Symbols { crate_name: module_name.into(), ..Default::default() };
    syms.modules.insert(String::new());
    syms.collect_items(&file.items, &[]);
    let mut ctx = FileCtx::new(&syms, src, opts, vec![], module_name.into());
    let mut out = ctx.lower_module(&file.attrs, &file.items);
    out.module.imports.retain(|i| i.module != ".");
    for r in out.records {
        fr.push(r);
    }
    fr.file_notes = out.file_notes;
    (print_module(&out.module), fr)
}

pub fn write_output(out: &Output, dir: &Path, with_runtime: bool, opts: &Options) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for (p, text) in &out.files {
        let full = dir.join(p);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(full, text)?;
    }
    if with_runtime {
        for (p, text) in RUNTIME_FILES {
            let full = dir.join(p);
            std::fs::create_dir_all(full.parent().unwrap())?;
            std::fs::write(full, text)?;
        }
    }
    std::fs::write(dir.join("coverage.json"), out.report.to_json())?;
    std::fs::write(dir.join("coverage.md"), out.report.summary(25))?;
    if opts.optimize_hints {
        std::fs::write(dir.join("hints.json"), serde_json::to_string_pretty(&out.hints).unwrap_or_default())?;
    }
    Ok(())
}

#[allow(dead_code)]
fn unused(_c: ChildMod) {}
