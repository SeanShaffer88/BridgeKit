# BridgeKit Roadmap

## v0.1.0 (current)
Binding generation and verification for the DLL collection, from each kit's own shipped header: `gen-cs` (a `public static partial class` of `[DllImport]` methods with exact signatures, `IntPtr` returns on the raw imports, and `<name>_string` wrappers that free caller-owned results through the kit's own `<prefix>_free_string` and never free borrowed ones), `gen-rs` (the `extern "C"` block, opaque handle structs, and the same wrappers), and `verify` (a pure-Rust PE export-table reader that fails — exit 1, naming each — on any header function missing from a built DLL, and warns on extra exports). The header grammar is pinned to the surveyed NotifyKit/WatchKit/SignKit dialect; unknown types are a hard error naming the token, never a guessed mapping. Golden tests pin both generators against trimmed copies of the three real headers; `verify` is tested against a byte-wise synthetic PE and was run green against SignKit's real shipped `sign_kit.dll` (7/7 exports).

## Next, in order
1. **Batch mode over the whole collection**: one command (`bridgekit gen-all --kits <dir> --lang cs|rs --out <dir>`) that discovers every kit's `include/*.h` under a directory, generates each binding with the kit's DLL name derived by convention (`<kit>` → `<kit>.dll` / repo name), and runs `verify` for every built DLL it finds beside the headers — turning "regenerate and re-verify the collection" from 33 command pairs into one gate step. Needs one decision first: the kit-name → DLL-file-name convention is not uniform across the oldest kits (it is `<prefix>_kit.dll` for the surveyed three), so the batch config must carry an explicit per-kit name map rather than deriving silently.
2. **Verify in the kits' CI**: a canonical snippet that runs `bridgekit verify` as a post-build step in each kit's `ci.yml`, so a header/export mismatch fails the kit's own CI instead of being found by a consumer.
3. **C header self-check**: parse the header a kit is *about to ship* in its gate (the parse alone, no generation), so a prototype outside the grammar is caught in the kit's repo rather than in BridgeKit's.

## Explicitly not planned
- Guessing mappings for types outside the grammar (function pointers, structs by value, `size_t`, ...): a guessed signature is worse than no signature — it compiles, loads, and lies at runtime. New types enter through the grammar, with tests.
- Parsing arbitrary C headers (system headers, macro-generated prototypes, `#include` chasing): BridgeKit reads the one header a kit ships, in the dialect the collection writes. It is not a C parser and must not grow into one.
- A C++ binding generator: no consumer in the collection binds the kits from C++ today; the shipped headers already serve C++ callers directly via `extern "C"`.
