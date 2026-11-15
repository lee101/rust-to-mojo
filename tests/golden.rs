//! Golden tests: every `tests/golden/NAME.rs` is translated and compared with
//! `NAME.mojo` (the generated Mojo) and `NAME.coverage.txt` (per-item status,
//! so honesty regressions fail too).
//!
//! Regenerate after an intentional change with:
//!     UPDATE_GOLDEN=1 cargo test --test golden
//! and review the diff by hand before committing.

use rust_to_mojo::report::FileReport;
use rust_to_mojo::{translate_source, Options};
use std::path::{Path, PathBuf};

fn coverage_text(r: &FileReport) -> String {
    let mut s = String::new();
    for i in &r.items {
        let st = format!("{:?}", i.status).to_lowercase();
        s.push_str(&format!("{} {} [line {}]: {}\n", i.kind, i.name, i.line, st));
        for reason in &i.reasons {
            s.push_str(&format!("    - {}\n", reason.key));
        }
    }
    let c = &r.counts;
    s.push_str(&format!(
        "total {} translated {} partial {} stubbed {} skipped {}\n",
        c.total, c.translated, c.partial, c.stubbed, c.skipped
    ));
    s
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("golden")
}

fn check(path: &Path, actual: &str, update: bool, failures: &mut Vec<String>) {
    if update {
        std::fs::write(path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(path).unwrap_or_default().replace("\r\n", "\n");
    if expected != actual {
        let mut diff = String::new();
        for (i, (a, b)) in expected.lines().zip(actual.lines()).enumerate() {
            if a != b {
                diff = format!("first difference at line {}:\n  expected: {}\n  actual:   {}", i + 1, a, b);
                break;
            }
        }
        if diff.is_empty() {
            diff = format!("line counts differ: expected {}, actual {}", expected.lines().count(), actual.lines().count());
        }
        failures.push(format!("{}: {}", path.display(), diff));
    }
}

#[test]
fn golden_files() {
    let update = std::env::var("UPDATE_GOLDEN").is_ok();
    let mut cases: Vec<PathBuf> = std::fs::read_dir(golden_dir())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "rs").unwrap_or(false))
        .collect();
    cases.sort();
    assert!(cases.len() >= 25, "expected at least 25 golden cases, found {}", cases.len());
    let mut failures = vec![];
    for case in &cases {
        let src = std::fs::read_to_string(case).unwrap().replace("\r\n", "\n");
        let stem = case.file_stem().unwrap().to_string_lossy().to_string();
        let opts = Options { optimize_hints: stem.contains("numeric") };
        let (mojo, report) = translate_source(&src, &stem, &opts);
        assert_eq!(report.parse_error, None, "{} failed to parse", stem);
        check(&case.with_extension("mojo"), &mojo, update, &mut failures);
        check(&case.with_extension("coverage.txt"), &coverage_text(&report), update, &mut failures);
    }
    if !failures.is_empty() {
        panic!("{} golden mismatches:\n{}", failures.len(), failures.join("\n"));
    }
}

/// Every stubbed item must leave a TODO marker with the original Rust.
#[test]
fn stubs_are_visible() {
    for case in std::fs::read_dir(golden_dir()).unwrap().filter_map(|e| e.ok().map(|e| e.path())) {
        if case.extension().map(|x| x != "rs").unwrap_or(true) {
            continue;
        }
        let src = std::fs::read_to_string(&case).unwrap();
        let stem = case.file_stem().unwrap().to_string_lossy().to_string();
        let (mojo, report) = translate_source(&src, &stem, &Options::default());
        let stubbed = report.items.iter().filter(|i| i.status == rust_to_mojo::report::Status::Stubbed).count();
        let markers = mojo.matches("not translated:").count();
        assert!(markers >= 1 || stubbed == 0, "{}: {} stubbed items but no stub markers", stem, stubbed);
        if stubbed > 0 {
            assert!(mojo.contains("--- original Rust ---"), "{}: stub without original source", stem);
        }
    }
}
