//! BridgeKit CLI — a thin shell over the library core.
//!
//! ```text
//! bridgekit gen-cs --header <kit.h> --dll-name <kit.dll> --out <file.cs>
//! bridgekit gen-rs --header <kit.h> --out <file.rs>
//! bridgekit verify --header <kit.h> --dll <kit.dll>
//! ```
//!
//! Exit codes: 0 success (for `verify`: every header function is
//! exported by the DLL), 1 a generation/verification failure, 2 a
//! usage error. All failure details go to stderr; `verify` also names
//! each missing export on stderr and each extra export as a warning.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    ExitCode::from(run(&args))
}

/// Run one command and return its process exit code. The subcommand
/// helpers use `Result<u8, u8>` (Ok/Err both carry the exit code) so
/// `?` can bail out of a deep step with the right code.
fn run(args: &[String]) -> u8 {
    let Some(cmd) = args.first() else {
        eprint!("{}", usage());
        return 2;
    };
    let code = match cmd.as_str() {
        "--version" | "-V" => {
            println!("bridgekit {}", bridgekit::VERSION);
            Ok(0)
        }
        "--help" | "-h" => {
            print!("{}", usage());
            Ok(0)
        }
        "gen-cs" => gen_cs_cmd(&args[1..]),
        "gen-rs" => gen_rs_cmd(&args[1..]),
        "verify" => verify_cmd(&args[1..]),
        other => {
            eprintln!("error: unknown command '{}'\n", other);
            eprint!("{}", usage());
            Err(2)
        }
    };
    code.unwrap_or_else(|c| c)
}

fn usage() -> String {
    format!(
        "bridgekit {} - verified bindings for the DLL collection, generated\n\
         from each kit's own shipped header (never hand-copied).\n\n\
         USAGE:\n  \
         bridgekit gen-cs --header <kit.h> --dll-name <kit.dll> --out <file.cs>\n  \
         bridgekit gen-rs --header <kit.h> --out <file.rs>\n  \
         bridgekit verify --header <kit.h> --dll <kit.dll>\n  \
         bridgekit --version\n\n\
         gen-cs / gen-rs write binding files whose signatures are copied exactly\n\
         from the header; unknown C types are a hard error naming the token.\n\
         verify parses the DLL's PE export table: exit 1 naming each function\n\
         the header declares but the DLL does not export; extra DLL exports\n\
         are warnings only.\n",
        bridgekit::VERSION
    )
}

/// Fetch `--flag value` from the argument list.
fn flag(args: &[String], name: &str) -> Result<String, u8> {
    let pos = args.iter().position(|a| a == name);
    match pos {
        Some(i) => match args.get(i + 1) {
            Some(v) if !v.starts_with("--") => Ok(v.clone()),
            _ => {
                eprintln!("error: {} needs a value", name);
                Err(2)
            }
        },
        None => {
            eprintln!("error: missing required {}", name);
            Err(2)
        }
    }
}

fn read_header(path: &str) -> Result<(String, bridgekit::header::Header), u8> {
    let name = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string());
    let text = fs::read_to_string(path).map_err(|e| {
        eprintln!("error: cannot read header '{}': {}", path, e);
        1u8
    })?;
    let header = bridgekit::header::parse_header(&text).map_err(|e| {
        eprintln!("error: {}: {}", path, e);
        1u8
    })?;
    Ok((name, header))
}

fn gen_cs_cmd(args: &[String]) -> Result<u8, u8> {
    let header_path = flag(args, "--header")?;
    let dll_name = flag(args, "--dll-name")?;
    let out_path = flag(args, "--out")?;
    let (name, header) = read_header(&header_path)?;
    let text = bridgekit::gen_cs::generate(&header, &name, &dll_name).map_err(|e| {
        eprintln!("error: {}", e);
        1u8
    })?;
    write_out(&out_path, &text)?;
    println!(
        "wrote {} ({} functions, class {})",
        out_path,
        header.functions.len(),
        bridgekit::gen_cs::class_name(&dll_name)
    );
    Ok(0)
}

fn gen_rs_cmd(args: &[String]) -> Result<u8, u8> {
    let header_path = flag(args, "--header")?;
    let out_path = flag(args, "--out")?;
    let (name, header) = read_header(&header_path)?;
    let text = bridgekit::gen_rs::generate(&header, &name).map_err(|e| {
        eprintln!("error: {}", e);
        1u8
    })?;
    write_out(&out_path, &text)?;
    println!("wrote {} ({} functions)", out_path, header.functions.len());
    Ok(0)
}

fn verify_cmd(args: &[String]) -> Result<u8, u8> {
    let header_path = flag(args, "--header")?;
    let dll_path = flag(args, "--dll")?;
    let (_name, header) = read_header(&header_path)?;
    let bytes = fs::read(&dll_path).map_err(|e| {
        eprintln!("error: cannot read DLL '{}': {}", dll_path, e);
        1u8
    })?;
    let report = bridgekit::verify(&header, &bytes).map_err(|e| {
        eprintln!("error: {}: {}", dll_path, e);
        1u8
    })?;
    for name in &report.extra {
        eprintln!("warning: extra export not declared in header: {}", name);
    }
    if report.passed() {
        println!(
            "ok: all {} header functions are exported by {} ({} named exports)",
            report.declared,
            PathBuf::from(&dll_path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or(dll_path.clone()),
            report.exports
        );
        Ok(0)
    } else {
        for name in &report.missing {
            eprintln!("missing export: {}", name);
        }
        eprintln!(
            "error: {} of {} header functions are missing from {}",
            report.missing.len(),
            report.declared,
            dll_path
        );
        Err(1)
    }
}

fn write_out(path: &str, text: &str) -> Result<(), u8> {
    fs::write(path, text).map_err(|e| {
        eprintln!("error: cannot write '{}': {}", path, e);
        1u8
    })
}
