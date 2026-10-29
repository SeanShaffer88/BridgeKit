//! Parser for the DLL collection's shipped C headers.
//!
//! The grammar is deliberately small — exactly what the kits' headers
//! use, surveyed against NotifyKit, WatchKit and SignKit:
//!
//! - Prototypes look like `RET prefix_name(params);`, optionally led
//!   by an export macro (`NO_API`, `WA_API`, `SG_API` — any all-caps
//!   token ending in `_API`), and may span several lines.
//! - Comments (`/* ... */` and `// ...`) and preprocessor lines are
//!   ignored. `typedef`, `enum` and `struct` statements are not
//!   prototypes and are skipped; the `extern "C"` wrapper is unwrapped.
//! - Types: `void`, `int`, `unsigned int`, `uint32_t`, `uint64_t`,
//!   `const char *`, `char *`, `char **`, `void *`, `const void *`,
//!   and opaque handles `Name *` / `Name **` where `Name` is a
//!   capitalized typedef'd struct name (`NoCenter`, `WaWatcher`, ...).
//! - Anything else is a hard error naming the offending token. A type
//!   mapping is never guessed: a guessed signature compiles, loads,
//!   and then lies at runtime, which is the failure this tool exists
//!   to prevent (hand-copied P/Invoke signatures have done exactly
//!   that in this collection's history).

/// One C type from the pinned grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CType {
    Void,
    Int,
    /// `unsigned int`
    UInt,
    /// `uint32_t`
    U32,
    /// `uint64_t`
    U64,
    /// `const char *`
    ConstCharPtr,
    /// `char *`
    CharPtr,
    /// `char **`
    CharPtrPtr,
    /// `void *`
    VoidPtr,
    /// `const void *`
    ConstVoidPtr,
    /// `Name *` — an opaque handle.
    HandlePtr(String),
    /// `Name **` — an out-pointer for a handle.
    HandlePtrPtr(String),
}

impl CType {
    /// True for the two string-return types (`char *` caller-owned,
    /// `const char *` borrowed).
    pub fn is_string_return(&self) -> bool {
        matches!(self, CType::CharPtr | CType::ConstCharPtr)
    }

    /// The handle type name, if this type is an opaque handle.
    pub fn handle_name(&self) -> Option<&str> {
        match self {
            CType::HandlePtr(n) | CType::HandlePtrPtr(n) => Some(n),
            _ => None,
        }
    }
}

/// One named function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: CType,
}

/// One parsed prototype.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub ret: CType,
    pub params: Vec<Param>,
}

/// A parsed header: the prototypes in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub functions: Vec<Function>,
}

impl Header {
    /// The kit's string-freeing function (`<prefix>_free_string`), if
    /// the header declares one. Generated wrappers free caller-owned
    /// strings through exactly this name — never an assumed one.
    pub fn free_string_fn(&self) -> Option<&str> {
        self.functions
            .iter()
            .map(|f| f.name.as_str())
            .find(|n| n.ends_with("_free_string"))
    }

    /// Every opaque handle type named by any prototype, in
    /// first-appearance order.
    pub fn handle_types(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for f in &self.functions {
            let tys = std::iter::once(&f.ret).chain(f.params.iter().map(|p| &p.ty));
            for ty in tys {
                if let Some(n) = ty.handle_name() {
                    if !out.contains(&n) {
                        out.push(n);
                    }
                }
            }
        }
        out
    }
}

/// Parse a header's text into its prototypes.
///
/// # Errors
/// Returns a message naming the offending token for any type outside
/// the pinned grammar, for a prototype that does not parse, or for a
/// duplicate prototype name.
pub fn parse_header(text: &str) -> Result<Header, String> {
    let no_comments = strip_comments(text);
    let no_preproc = strip_preprocessor_lines(&no_comments);
    let mut functions = Vec::new();
    for stmt in no_preproc.split(';') {
        if let Some(f) = parse_statement(stmt)? {
            if functions.iter().any(|g: &Function| g.name == f.name) {
                return Err(format!("duplicate prototype for '{}'", f.name));
            }
            functions.push(f);
        }
    }
    Ok(Header { functions })
}

/// Remove `/* ... */` and `// ...` comments. String literals are not
/// tracked: the collection's headers contain no comment markers inside
/// string literals (the only literal is `"C"` in `extern "C"`).
fn strip_comments(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // Block comment: replace with a space so tokens on either
            // side never fuse.
            out.push(' ');
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
        } else if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            out.push(' ');
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else {
            // Push the whole char (headers are ASCII in practice, but
            // never split a UTF-8 sequence).
            let ch_len = utf8_len(bytes[i]);
            out.push_str(&text[i..(i + ch_len).min(text.len())]);
            i += ch_len;
        }
    }
    out
}

fn utf8_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first >> 5 == 0b110 {
        2
    } else if first >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

/// Drop lines whose first non-whitespace character is `#`
/// (`#include`, `#define`, the include guards, `#ifdef` blocks).
fn strip_preprocessor_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Parse one `;`-terminated statement. Returns `None` for statements
/// that are not prototypes (typedefs, enums, structs, the braces of
/// the `extern "C"` wrapper, stray fragments).
fn parse_statement(stmt: &str) -> Result<Option<Function>, String> {
    let mut s = stmt.trim();
    if s.is_empty() {
        return Ok(None);
    }
    // Unwrap a leading `extern "C" {` (or an enum/struct opener): if
    // the text before the first '{' holds no '(', it is a wrapper or a
    // definition opener, not part of a prototype — cut through it.
    if let Some(brace) = s.find('{') {
        let head = &s[..brace];
        if !head.contains('(') {
            s = s[brace + 1..].trim();
        }
    }
    if s.contains('{') || s.contains('}') {
        return Ok(None); // enum/struct body or the extern closing brace
    }
    if s.starts_with("typedef") {
        return Ok(None);
    }
    let Some(open) = s.find('(') else {
        return Ok(None); // not a prototype (macro remnant, stray text)
    };
    if !s.ends_with(')') {
        return Err(format!("cannot parse declaration: '{}'", one_line(s)));
    }
    let close = s.len() - 1; // the final ')'
    let head_tokens = tokenize(&s[..open]);
    let head_tokens = drop_export_macros(head_tokens);
    let Some((name, ret_tokens)) = head_tokens.split_last() else {
        return Err(format!("cannot parse declaration: '{}'", one_line(s)));
    };
    if !is_identifier(name) {
        return Err(format!(
            "cannot parse declaration: '{}' has no function name",
            one_line(s)
        ));
    }
    let ret = classify(ret_tokens).map_err(|tok| unknown_type(&tok, name, "return type"))?;

    let params_text = s[open + 1..close].trim();
    let mut params = Vec::new();
    if !params_text.is_empty() && params_text != "void" {
        for part in params_text.split(',') {
            let part = part.trim();
            if part.contains('(') || part.contains(')') {
                return Err(format!(
                    "unsupported parameter '{}' in prototype '{}': nested declarators are outside the header grammar",
                    one_line(part),
                    name
                ));
            }
            let tokens = drop_export_macros(tokenize(part));
            let Some((pname, ptokens)) = tokens.split_last() else {
                return Err(format!("empty parameter in prototype '{}'", name));
            };
            if !is_identifier(pname) {
                return Err(format!(
                    "parameter '{}' in prototype '{}' has no name",
                    one_line(part),
                    name
                ));
            }
            if ptokens.is_empty() {
                return Err(format!(
                    "parameter '{}' in prototype '{}' has no type",
                    pname, name
                ));
            }
            let ty = classify(ptokens)
                .map_err(|tok| unknown_type(&tok, name, &format!("parameter '{}'", pname)))?;
            params.push(Param {
                name: pname.clone(),
                ty,
            });
        }
    }
    Ok(Some(Function {
        name: name.clone(),
        ret,
        params,
    }))
}

/// Split a declaration fragment into tokens: identifiers/numbers stay
/// whole, `*` is always its own token.
fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        if ch == '*' {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            out.push("*".to_string());
        } else if ch.is_whitespace() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Export macros are all-caps tokens ending in `_API`
/// (`NO_API`, `WA_API`, `SG_API`). They carry no type information.
fn drop_export_macros(tokens: Vec<String>) -> Vec<String> {
    tokens
        .into_iter()
        .filter(|t| {
            !(t.len() > 4
                && t.ends_with("_API")
                && t.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'))
        })
        .collect()
}

fn is_identifier(tok: &str) -> bool {
    let mut chars = tok.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

/// Map a token list to a grammar type. On failure returns the first
/// token that is not part of any known type, so the error can name it.
fn classify(tokens: &[String]) -> Result<CType, String> {
    let toks: Vec<&str> = tokens.iter().map(String::as_str).collect();
    let ty = match toks.as_slice() {
        ["void"] => CType::Void,
        ["int"] => CType::Int,
        ["unsigned", "int"] => CType::UInt,
        ["uint32_t"] => CType::U32,
        ["uint64_t"] => CType::U64,
        ["const", "char", "*"] => CType::ConstCharPtr,
        ["char", "*"] => CType::CharPtr,
        ["char", "*", "*"] => CType::CharPtrPtr,
        ["void", "*"] => CType::VoidPtr,
        ["const", "void", "*"] => CType::ConstVoidPtr,
        [name, "*"] if is_handle_name(name) => CType::HandlePtr((*name).to_string()),
        [name, "*", "*"] if is_handle_name(name) => CType::HandlePtrPtr((*name).to_string()),
        _ => {
            const KNOWN: [&str; 8] = [
                "const", "char", "void", "int", "unsigned", "uint32_t", "uint64_t", "*",
            ];
            let bad = toks
                .iter()
                .find(|t| !KNOWN.contains(t) && !is_handle_name(t))
                .copied()
                .unwrap_or_else(|| toks.first().copied().unwrap_or(""));
            return Err(bad.to_string());
        }
    };
    Ok(ty)
}

/// Opaque handles are capitalized typedef'd struct names
/// (`NoCenter`, `WaWatcher`). Requiring the capital keeps a lowercase
/// unknown type like `size_t *` an error instead of a "handle".
fn is_handle_name(tok: &str) -> bool {
    is_identifier(tok) && tok.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

fn unknown_type(token: &str, fn_name: &str, what: &str) -> String {
    format!(
        "unknown type token '{}' in {} of prototype '{}': BridgeKit never guesses a mapping — extend the header grammar in src/header.rs if this type is real",
        token, what, fn_name
    )
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_core_shapes() {
        let h = parse_header(
            "XX_API const char *xx_version(void);\n\
             XX_API int xx_do(Thing *t, const char *json, char **out, uint64_t n, unsigned int flags);\n\
             XX_API void xx_free_string(char *p);\n",
        )
        .unwrap();
        assert_eq!(h.functions.len(), 3);
        assert_eq!(h.functions[0].ret, CType::ConstCharPtr);
        assert_eq!(h.functions[1].params.len(), 5);
        assert_eq!(h.functions[1].params[1].ty, CType::ConstCharPtr);
        assert_eq!(h.functions[1].params[2].ty, CType::CharPtrPtr);
        assert_eq!(h.functions[1].params[3].ty, CType::U64);
        assert_eq!(h.functions[1].params[4].ty, CType::UInt);
        assert_eq!(h.free_string_fn(), Some("xx_free_string"));
        assert_eq!(h.handle_types(), vec!["Thing"]);
    }

    #[test]
    fn skips_typedefs_enums_and_extern_wrappers() {
        let h = parse_header(
            "#ifndef X_H\n#define X_H\n\
             typedef struct Thing Thing;\n\
             enum { XX_OK = 0, XX_ERR = 1 };\n\
             extern \"C\" {\n\
             XX_API int xx_count(Thing *t);\n\
             }\n#endif\n",
        )
        .unwrap();
        assert_eq!(h.functions.len(), 1);
        assert_eq!(h.functions[0].name, "xx_count");
    }

    #[test]
    fn unknown_type_names_the_token() {
        let err = parse_header("XX_API int xx_bad(float factor);").unwrap_err();
        assert!(err.contains("'float'"), "error was: {err}");
        let err = parse_header("XX_API size_t xx_len(void);").unwrap_err();
        assert!(err.contains("'size_t'"), "error was: {err}");
    }

    #[test]
    fn multiline_prototype_parses() {
        let h = parse_header("XX_API int xx_long(Thing *t,\n               const char *json,\n               char **out);\n").unwrap();
        assert_eq!(h.functions[0].params.len(), 3);
    }
}
