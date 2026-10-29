//! Generation tests: golden outputs for both generators over the
//! three real-kit fixtures, plus the grammar's failure modes.

use bridgekit::header::parse_header;
use std::path::PathBuf;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {:?}: {}", path, e))
}

fn golden(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {:?}: {}", path, e));
    text.replace("\r\n", "\n")
}

#[test]
fn golden_cs_notifykit() {
    let header = parse_header(&fixture("notifykit.h")).unwrap();
    let out = bridgekit::gen_cs::generate(&header, "notifykit.h", "notify_kit.dll").unwrap();
    assert_eq!(out, golden("notifykit.cs"));
}

#[test]
fn golden_cs_signkit() {
    let header = parse_header(&fixture("signkit.h")).unwrap();
    let out = bridgekit::gen_cs::generate(&header, "signkit.h", "sign_kit.dll").unwrap();
    assert_eq!(out, golden("signkit.cs"));
}

#[test]
fn golden_rs_watchkit() {
    let header = parse_header(&fixture("watchkit.h")).unwrap();
    let out = bridgekit::gen_rs::generate(&header, "watchkit.h").unwrap();
    assert_eq!(out, golden("watchkit.rs"));
}

#[test]
fn golden_rs_notifykit() {
    let header = parse_header(&fixture("notifykit.h")).unwrap();
    let out = bridgekit::gen_rs::generate(&header, "notifykit.h").unwrap();
    assert_eq!(out, golden("notifykit.rs"));
}

#[test]
fn fixtures_parse_to_the_expected_shapes() {
    let no = parse_header(&fixture("notifykit.h")).unwrap();
    assert_eq!(no.functions.len(), 10);
    assert_eq!(no.handle_types(), vec!["NoCenter", "NoTray"]);
    assert_eq!(no.free_string_fn(), Some("no_free_string"));

    let wa = parse_header(&fixture("watchkit.h")).unwrap();
    assert_eq!(wa.functions.len(), 8);
    assert_eq!(wa.handle_types(), vec!["WaWatcher"]);

    let sg = parse_header(&fixture("signkit.h")).unwrap();
    assert_eq!(sg.functions.len(), 7);
    assert!(sg.handle_types().is_empty());
}

#[test]
fn cs_class_is_always_public() {
    // The historical failure: non-public [DllImport] members compile
    // and then fail at call time. Pin the generated shape.
    let header = parse_header(&fixture("signkit.h")).unwrap();
    let out = bridgekit::gen_cs::generate(&header, "signkit.h", "sign_kit.dll").unwrap();
    assert!(out.contains("public static partial class SignKit"));
    assert!(out.contains("public static extern"));
    assert!(!out.contains("internal static"));
}

#[test]
fn unknown_type_is_a_hard_error_naming_the_token() {
    let err = parse_header(&fixture("badtype.h")).unwrap_err();
    assert!(err.contains("'float'"), "error was: {err}");
    assert!(err.contains("bk_scale"), "error was: {err}");

    let err = parse_header("BK_API double bk_measure(BkThing *t);").unwrap_err();
    assert!(err.contains("'double'"), "error was: {err}");

    let err = parse_header("BK_API int bk_set(size_t count);").unwrap_err();
    assert!(err.contains("'size_t'"), "error was: {err}");
}

#[test]
fn owned_string_result_without_free_string_is_an_error() {
    // A char* result with no declared <prefix>_free_string cannot be
    // wrapped safely; both generators must refuse, not improvise.
    let header = parse_header("BK_API char *bk_make(const char *name);").unwrap();
    let err = bridgekit::gen_cs::generate(&header, "badtype.h", "bad_kit.dll").unwrap_err();
    assert!(err.contains("free_string"), "error was: {err}");
    let err = bridgekit::gen_rs::generate(&header, "badtype.h").unwrap_err();
    assert!(err.contains("free_string"), "error was: {err}");
}
