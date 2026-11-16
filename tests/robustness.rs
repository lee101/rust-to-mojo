//! The translator must never panic, whatever the input; translation failures
//! are reported, not thrown.

use rust_to_mojo::{translate_path, translate_source, Options};
use std::path::Path;

const NASTY: &[&str] = &[
    "",
    "fn",
    "struct S<'a, T: ?Sized + 'a>(&'a T);",
    "fn f() { let x = || -> i32 { loop { break 5 } }; }",
    "impl<T> Foo for T where for<'a> &'a T: Into<u8> {}",
    "fn g<const N: usize>(a: [u8; N]) -> [u8; N] { a }",
    "enum E { A(Box<E>), B }",
    "fn h(x: &mut dyn Iterator<Item = u8>) -> Option<u8> { x.next() }",
    "macro_rules! m { () => {} } m!();",
    "fn k() -> impl Fn(u8) -> u8 { |x| x }",
    "const C: [u8; 3] = [1, 2, 3]; static S: &str = \"hi\";",
    "fn deep() { if let Some(Some((a, [b, ..]))) = None::<Option<(u8, [u8; 2])>> {} }",
    "type A<T> = std::collections::BTreeMap<T, Vec<T>>;",
    "fn q() -> Result<(), Box<dyn std::error::Error>> { let _x: u8 = \"3\".parse()?; Ok(()) }",
    "pub(crate) mod inner { pub fn f() {} } use inner::f;",
    "trait T { fn f(&self) -> Self where Self: Sized; }",
    "fn tup() -> (u8, (u16, u32)) { let t = (1, (2, 3)); t.1 .0; t }",
];

#[test]
fn never_panics_on_odd_inputs() {
    for (i, src) in NASTY.iter().enumerate() {
        let r = std::panic::catch_unwind(|| translate_source(src, &format!("case{}", i), &Options { optimize_hints: true }));
        assert!(r.is_ok(), "translator panicked on case {}: {:?}", i, src);
        let (_, report) = r.unwrap();
        // internal errors are caught per item; make them visible here
        for it in &report.items {
            for reason in &it.reasons {
                assert!(!reason.key.starts_with("internal error"), "case {} ({}): {}", i, src, reason.detail);
            }
        }
    }
}

#[test]
fn crate_mode_layout() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("minicrate");
    let out = translate_path(&dir, &Options::default()).expect("crate translates");
    let paths: Vec<String> = out.files.iter().map(|(p, _)| p.to_string_lossy().replace('\\', "/")).collect();
    for want in ["minicrate/__init__.mojo", "minicrate/geometry.mojo", "minicrate/color/__init__.mojo", "minicrate/color/hsla.mojo", "minicrate/util.mojo"] {
        assert!(paths.contains(&want.to_string()), "missing {} in {:?}", want, paths);
    }
    let geometry = &out.files.iter().find(|(p, _)| p.ends_with("geometry.mojo")).unwrap().1;
    assert!(geometry.contains("struct Point"), "{}", geometry);
    let hsla = &out.files.iter().find(|(p, _)| p.ends_with("hsla.mojo")).unwrap().1;
    assert!(hsla.contains("from minicrate.geometry import Point"), "{}", hsla);
    // `impl Point` written in color/hsla.rs is merged into the struct in geometry.mojo
    assert!(geometry.contains("def to_hsla(self) -> Hsla:"), "{}", geometry);
    assert!(geometry.contains("from minicrate.color.hsla import Hsla, at"), "{}", geometry);
    assert!(hsla.contains("is translated in `minicrate.geometry`"), "{}", hsla);
    let root = &out.files.iter().find(|(p, _)| p.to_string_lossy().replace('\\', "/") == "minicrate/__init__.mojo").unwrap().1;
    assert!(root.contains("from . import color"), "{}", root);
    assert_eq!(out.report.internal_errors, 0);
    assert!(out.report.counts.total >= 6);
    // the external-crate import is reported, not silently dropped
    assert!(out.report.file_notes.iter().any(|n| n.key.contains("external crate `serde`")), "{:?}", out.report.file_notes);
}
