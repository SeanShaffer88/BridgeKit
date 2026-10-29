//! Verify tests: header-vs-export-table comparison against a
//! synthetic PE built byte-wise in `tests/common` — matching,
//! missing-export, and extra-export cases, plus malformed inputs.

mod common;

use bridgekit::header::parse_header;

fn signkit_header() -> bridgekit::header::Header {
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/signkit.h");
    let text = std::fs::read_to_string(path).unwrap();
    parse_header(&text).unwrap()
}

fn signkit_names() -> Vec<&'static str> {
    vec![
        "sg_version",
        "sg_last_error",
        "sg_last_error_code",
        "sg_free_string",
        "sg_inspect",
        "sg_authenticode_hash",
        "sg_verify",
    ]
}

#[test]
fn synthetic_pe_exports_read_back() {
    let pe = common::build_pe(&signkit_names());
    let names = bridgekit::pe::export_names(&pe).unwrap();
    // The builder stores the name table sorted (as a real linker
    // emits it), so compare against the sorted declaration list.
    let mut expected = signkit_names();
    expected.sort_unstable();
    assert_eq!(names, expected);
}

#[test]
fn matching_dll_passes() {
    let pe = common::build_pe(&signkit_names());
    let report = bridgekit::verify(&signkit_header(), &pe).unwrap();
    assert!(report.passed());
    assert_eq!(report.declared, 7);
    assert_eq!(report.exports, 7);
    assert!(report.missing.is_empty());
    assert!(report.extra.is_empty());
}

#[test]
fn missing_export_fails_and_is_named() {
    let mut names = signkit_names();
    names.retain(|n| *n != "sg_verify");
    let pe = common::build_pe(&names);
    let report = bridgekit::verify(&signkit_header(), &pe).unwrap();
    assert!(!report.passed());
    assert_eq!(report.missing, vec!["sg_verify".to_string()]);
}

#[test]
fn extra_export_is_a_warning_not_a_failure() {
    let mut names = signkit_names();
    names.push("sg_internal_helper");
    let pe = common::build_pe(&names);
    let report = bridgekit::verify(&signkit_header(), &pe).unwrap();
    assert!(report.passed());
    assert_eq!(report.extra, vec!["sg_internal_helper".to_string()]);
}

#[test]
fn empty_export_table_reports_everything_missing() {
    let pe = common::build_pe(&[]);
    let report = bridgekit::verify(&signkit_header(), &pe).unwrap();
    assert!(!report.passed());
    assert_eq!(report.missing.len(), 7);
}

#[test]
fn non_pe_and_truncated_inputs_are_errors_not_panics() {
    let header = signkit_header();
    let err = bridgekit::verify(&header, b"this is not a PE").unwrap_err();
    assert!(err.contains("not a PE"), "error was: {err}");

    let mut pe = common::build_pe(&signkit_names());
    pe.truncate(0x100); // headers intact, export data gone
    assert!(bridgekit::verify(&header, &pe).is_err());
}
