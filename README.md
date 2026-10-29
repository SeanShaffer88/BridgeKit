# BridgeKit

## TL;DR

BridgeKit writes the boring, error-prone glue code between a Windows app and the DLL collection — the C# `[DllImport]` declarations and the Rust `extern "C"` blocks — *for* you, straight from each kit's own shipped C header. Nobody retypes a signature by hand ever again, so a signature can never quietly drift from the DLL it calls into: if the header changes, you regenerate, and if a built DLL doesn't match its header, `bridgekit verify` says so and names the missing functions. Think of it as a photocopier for declarations, with a checker that compares the copy against the real DLL.

```console
$ bridgekit gen-cs --header include/notifykit.h --dll-name notify_kit.dll --out NotifyKit.cs
wrote NotifyKit.cs (15 functions, class NotifyKit)

$ bridgekit verify --header include/signkit.h --dll sign_kit.dll
ok: all 7 header functions are exported by sign_kit.dll (7 named exports)
```

## Why this exists

Every consumer of the collection used to copy the kits' function signatures by hand — into C# P/Invoke declarations, into Rust `extern` blocks, into test harnesses. Hand copies drift, and they fail in the worst ways:

- A copied signature that declares a caller-owned `char *` result as a managed `string` frees memory with the wrong allocator — corruption, not an error message.
- A subtly wrong parameter shape returns empty strings instead of the real error, sending debugging in the wrong direction entirely.
- A binding class that isn't `public` compiles happily and then fails a whole round of smoke tests with inaccessible-member errors.

Generating the bindings from the shipped header removes the copy step completely, and `verify` closes the loop against the compiled DLL: a header that promises a function the DLL doesn't export is caught by name, before any app ships against it.

## Usage

### Generate C# bindings

```console
bridgekit gen-cs --header <kit.h> --dll-name <kit.dll> --out <file.cs>
```

Emits a `public static partial class` (named after the DLL: `notify_kit.dll` → `NotifyKit`) in the `BridgeKit.Generated` namespace:

- One `[DllImport]` method per prototype, with the signature copied exactly: `const char *` parameters marshal as `string`; handles, pointers and `char *` parameters are `IntPtr`; out-pointers (`char **`, `Handle **`) are `out IntPtr`.
- String *returns* stay `IntPtr` on the raw import — deliberately. Each string-returning function also gets a `<name>_string` wrapper that copies the UTF-8 text with `Marshal.PtrToStringUTF8`. For caller-owned `char *` results the wrapper frees the original through the kit's own `<prefix>_free_string` (in a `finally`); for borrowed `const char *` results (`<prefix>_version`, `<prefix>_last_error`) it copies and never frees.
- The class is always `public` — see "Why this exists".

The file is a fragment meant to be dropped into the consuming project as-is; regenerate it, never edit it.

### Generate Rust bindings

```console
bridgekit gen-rs --header <kit.h> --out <file.rs>
```

Emits the `extern "C"` block with exact signatures, one `#[repr(C)]` opaque struct per handle type, only the `std::os::raw` imports the signatures actually use, and a `<name>_string` wrapper per string-returning function (same owned/borrowed rules as the C# side). Edition-2021 style, matching the family crates.

### Verify a built DLL against its header

```console
bridgekit verify --header <kit.h> --dll <kit.dll>
```

Parses the DLL's PE export table and compares it with the header:

- Every function the header declares but the DLL does not export is named on stderr (`missing export: <name>`) and the exit code is **1**.
- Exports the DLL carries but the header does not declare are warnings only (`warning: extra export not declared in header: <name>`) — they never fail the check.
- All present: exit **0** with a one-line summary.

## The header grammar

BridgeKit parses exactly the header dialect the collection ships — surveyed against NotifyKit, WatchKit and SignKit — and nothing more:

- Prototypes `RET prefix_name(params);`, optionally led by an export macro (`NO_API`, `WA_API`, ...), possibly spanning lines. Comments and preprocessor lines are ignored; `typedef` / `enum` / `struct` statements are skipped; the `extern "C"` wrapper is unwrapped.
- Types: `void`, `int`, `unsigned int`, `uint32_t`, `uint64_t`, `const char *`, `char *`, `char **`, `void *`, `const void *`, and opaque handles `Name *` / `Name **` where `Name` is a capitalized typedef'd struct name.
- **Anything else is a hard error naming the offending token.** BridgeKit never guesses a mapping: a guessed signature compiles, loads, and lies at runtime. If a kit grows a genuinely new type, extend the grammar in `src/header.rs` with its test in the same change.

## Notes and limits

- Generation is deterministic: the same header produces byte-identical output (the test suite pins golden files generated from trimmed copies of the three surveyed kits' real headers).
- `verify` reads named exports only; ordinal-only exports cannot be matched to a header and are ignored.
- The generated C# marshals `string` parameters with the default (ANSI) P/Invoke charset, which is what the kits' consumers use for their JSON payloads today; returned strings are always read as UTF-8.
- Windows runtime behaviour of the *generated bindings* belongs to the consuming app to field-test — BridgeKit's own evidence is the golden corpus plus `verify` runs against real built DLLs (SignKit's shipped `sign_kit.dll` passes 7/7).

## Building

Zero-dependency Rust (edition 2021, rust-version 1.80):

```console
cargo build --release   # produces the bridgekit binary
cargo test              # full suite: parser, goldens, synthetic-PE verify, CLI
```

Quality gate: the WinDevKit gate (`tools/gate.sh`: fmt + clippy on host and Windows targets + the full test suite). Releases attach one zip, `BridgeKit-<tag>-x64.zip`, containing `bridgekit.exe`, `README.md` and `LICENSE`. Binaries are unsigned.

## License

MIT — see `LICENSE`.
