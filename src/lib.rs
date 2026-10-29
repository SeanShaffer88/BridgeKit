//! BridgeKit — verified binding generation for the DLL collection.
//!
//! The collection's apps kept hand-copying P/Invoke / `extern "C"`
//! signatures from the kits' headers, and hand copies drift: in this
//! project's history a wrong copied signature returned empty strings
//! instead of errors, and a set of non-public `[DllImport]` members
//! failed a whole smoke-test round. BridgeKit removes the copying:
//! bindings are *generated* from the header each kit actually ships
//! (`include/<kit>.h`), and `bridgekit verify` cross-checks a built
//! DLL's export table against that header, so a signature can only
//! drift if the header itself changes — at which point regeneration
//! is one command.
//!
//! Three operations, all pure functions over bytes/text (the CLI in
//! `main.rs` is a thin shell):
//!
//! - [`gen_cs::generate`] — a `public static partial class` of
//!   `[DllImport]` methods plus safe string wrappers.
//! - [`gen_rs::generate`] — the `extern "C"` block, opaque handle
//!   structs, and safe string wrappers.
//! - [`verify`] — header-vs-DLL export comparison over [`pe`].
//!
//! The header grammar is pinned in [`header`]: only the types the
//! collection's headers actually use are mapped, and anything else is
//! a hard error naming the token. A guessed mapping is worse than no
//! mapping — it compiles, loads, and lies at runtime.

pub mod gen_cs;
pub mod gen_rs;
pub mod header;
pub mod pe;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The outcome of comparing a header against a DLL's export table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyReport {
    /// Functions the header declares.
    pub declared: usize,
    /// Named exports the DLL carries.
    pub exports: usize,
    /// Declared in the header but missing from the DLL (a failure).
    pub missing: Vec<String>,
    /// Exported by the DLL but not declared in the header (a warning).
    pub extra: Vec<String>,
}

impl VerifyReport {
    /// Verification passes when nothing declared is missing. Extra
    /// exports are reported but never fail the check.
    pub fn passed(&self) -> bool {
        self.missing.is_empty()
    }
}

/// Compare a parsed header against a DLL's bytes.
///
/// # Errors
/// Propagates the PE parser's error if `dll_bytes` is not a readable
/// PE with an export table.
pub fn verify(header: &header::Header, dll_bytes: &[u8]) -> Result<VerifyReport, String> {
    let exports = pe::export_names(dll_bytes)?;
    let missing: Vec<String> = header
        .functions
        .iter()
        .map(|f| f.name.clone())
        .filter(|n| !exports.contains(n))
        .collect();
    let extra: Vec<String> = exports
        .iter()
        .filter(|e| !header.functions.iter().any(|f| &f.name == *e))
        .cloned()
        .collect();
    Ok(VerifyReport {
        declared: header.functions.len(),
        exports: exports.len(),
        missing,
        extra,
    })
}
