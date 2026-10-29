//! CLI tests: the real `bridgekit` binary end-to-end — generated
//! file content, and `verify` exit codes (0 match, 1 missing, with
//! the missing export named on stderr).

mod common;

use std::path::PathBuf;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bridgekit"))
        .args(args)
        .output()
        .expect("run bridgekit binary")
}

fn fixture_path(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn temp_file(tag: &str, name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bridgekit-cli-{}-{}", std::process::id(), tag));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn cli_gen_cs_matches_the_golden() {
    let out = temp_file("gencs", "NotifyKit.cs");
    let output = run(&[
        "gen-cs",
        "--header",
        &fixture_path("notifykit.h"),
        "--dll-name",
        "notify_kit.dll",
        "--out",
        &out.to_string_lossy(),
    ]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let produced = std::fs::read_to_string(&out).unwrap().replace("\r\n", "\n");
    let golden_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/notifykit.cs");
    let golden = std::fs::read_to_string(golden_path)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(produced, golden);
}

#[test]
fn cli_verify_exit_codes_and_messages() {
    // Matching exports -> exit 0.
    let names: Vec<&str> = vec![
        "sg_version",
        "sg_last_error",
        "sg_last_error_code",
        "sg_free_string",
        "sg_inspect",
        "sg_authenticode_hash",
        "sg_verify",
    ];
    let dll = temp_file("verify", "sign_kit.dll");
    std::fs::write(&dll, common::build_pe(&names)).unwrap();
    let output = run(&[
        "verify",
        "--header",
        &fixture_path("signkit.h"),
        "--dll",
        &dll.to_string_lossy(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("ok:"));

    // One missing export -> exit 1, and stderr names it.
    let short: Vec<&str> = names
        .iter()
        .copied()
        .filter(|n| *n != "sg_verify")
        .collect();
    std::fs::write(&dll, common::build_pe(&short)).unwrap();
    let output = run(&[
        "verify",
        "--header",
        &fixture_path("signkit.h"),
        "--dll",
        &dll.to_string_lossy(),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("missing export: sg_verify"),
        "stderr was: {stderr}"
    );
}

#[test]
fn cli_unknown_type_exits_1_and_names_the_token() {
    let out = temp_file("badtype", "Bad.cs");
    let output = run(&[
        "gen-cs",
        "--header",
        &fixture_path("badtype.h"),
        "--dll-name",
        "bad_kit.dll",
        "--out",
        &out.to_string_lossy(),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("'float'"), "stderr was: {stderr}");
    assert!(!out.exists(), "no output file may be written on failure");
}
