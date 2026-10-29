//! Shared test helper: build a minimal synthetic PE32+ image
//! byte-by-byte, with a correct export directory carrying exactly
//! the given names. Lets `verify` be tested end-to-end without a
//! Windows toolchain or a fixture DLL binary checked into the repo.

/// File offset where the PE signature sits (`e_lfanew`).
const PE_OFF: usize = 0x80;
/// RVA of the single section (and of the export blob inside it).
const EXPORT_RVA: u32 = 0x1000;
/// File offset of the section's raw data.
const RAW_OFF: usize = 0x200;
/// PE32+ optional header size.
const OPT_SIZE: usize = 0xF0;

fn put_u16(buf: &mut [u8], off: usize, v: u16) {
    buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

fn put_u32(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

/// A minimal PE32+ DLL image whose export table names `exports`
/// (stored sorted, as a real linker emits them).
pub fn build_pe(exports: &[&str]) -> Vec<u8> {
    let mut names: Vec<&str> = exports.to_vec();
    names.sort_unstable();
    let n = names.len();

    // Export blob: directory (40 bytes), then the three tables, then
    // the strings ("synthetic.dll" first, then each export name).
    let funcs_off = 40usize;
    let names_off = funcs_off + 4 * n;
    let ords_off = names_off + 4 * n;
    let mut blob = vec![0u8; ords_off + 2 * n];
    let mut cursor = blob.len();
    let dll_name_rva = EXPORT_RVA + cursor as u32;
    blob.extend_from_slice(b"synthetic.dll\0");
    cursor += "synthetic.dll\0".len();
    let mut name_rvas = Vec::with_capacity(n);
    for name in &names {
        name_rvas.push(EXPORT_RVA + cursor as u32);
        blob.extend_from_slice(name.as_bytes());
        blob.push(0);
        cursor += name.len() + 1;
    }
    put_u32(&mut blob, 12, dll_name_rva); // Name
    put_u32(&mut blob, 16, 1); // Base
    put_u32(&mut blob, 20, n as u32); // NumberOfFunctions
    put_u32(&mut blob, 24, n as u32); // NumberOfNames
    put_u32(&mut blob, 28, EXPORT_RVA + funcs_off as u32); // AddressOfFunctions
    put_u32(&mut blob, 32, EXPORT_RVA + names_off as u32); // AddressOfNames
    put_u32(&mut blob, 36, EXPORT_RVA + ords_off as u32); // AddressOfNameOrdinals
    for (i, rva) in name_rvas.iter().enumerate() {
        // Function RVAs are dummies: the name table is what verify
        // reads, and a real table's code RVAs are never followed.
        put_u32(&mut blob, funcs_off + 4 * i, EXPORT_RVA);
        put_u32(&mut blob, names_off + 4 * i, *rva);
        put_u16(&mut blob, ords_off + 2 * i, i as u16);
    }

    let mut img = vec![0u8; RAW_OFF + blob.len()];
    img[0] = b'M';
    img[1] = b'Z';
    put_u32(&mut img, 0x3C, PE_OFF as u32); // e_lfanew
    img[PE_OFF..PE_OFF + 4].copy_from_slice(b"PE\0\0");
    let coff = PE_OFF + 4;
    put_u16(&mut img, coff, 0x8664); // Machine: x86-64
    put_u16(&mut img, coff + 2, 1); // NumberOfSections
    put_u16(&mut img, coff + 16, OPT_SIZE as u16); // SizeOfOptionalHeader
    let opt = coff + 20;
    put_u16(&mut img, opt, 0x20B); // Magic: PE32+
    put_u32(&mut img, opt + 108, 16); // NumberOfRvaAndSizes
    put_u32(&mut img, opt + 112, EXPORT_RVA); // Export dir RVA
    put_u32(&mut img, opt + 116, blob.len() as u32); // Export dir size
    let sec = opt + OPT_SIZE;
    img[sec..sec + 8].copy_from_slice(b".edata\0\0");
    put_u32(&mut img, sec + 8, blob.len() as u32); // VirtualSize
    put_u32(&mut img, sec + 12, EXPORT_RVA); // VirtualAddress
    put_u32(&mut img, sec + 16, blob.len() as u32); // SizeOfRawData
    put_u32(&mut img, sec + 20, RAW_OFF as u32); // PointerToRawData
    img[RAW_OFF..RAW_OFF + blob.len()].copy_from_slice(&blob);
    img
}
