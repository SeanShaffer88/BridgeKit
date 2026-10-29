//! Minimal PE export-table reader, used by `bridgekit verify`.
//!
//! Pure byte parsing (bounds-checked throughout, no panics): reads
//! the DOS header, the PE signature, the COFF header, the optional
//! header's export data directory, and the section table, then walks
//! the export name pointer table. Only exports *with names* are
//! returned — ordinal-only exports cannot be matched against a header
//! and are ignored by the comparison.

/// Read the named exports of a PE image (DLL or EXE).
///
/// # Errors
/// A message saying what was wrong (not a PE, truncated structures,
/// an RVA that maps nowhere) — never a panic.
pub fn export_names(bytes: &[u8]) -> Result<Vec<String>, String> {
    if bytes.len() < 0x40 || &bytes[0..2] != b"MZ" {
        return Err("not a PE file (missing MZ signature)".to_string());
    }
    let pe_off = rd_u32(bytes, 0x3C)? as usize;
    if rd_bytes(bytes, pe_off, 4)? != b"PE\0\0" {
        return Err("not a PE file (missing PE signature)".to_string());
    }
    let coff = pe_off + 4;
    let num_sections = rd_u16(bytes, coff + 2)? as usize;
    let opt_size = rd_u16(bytes, coff + 16)? as usize;
    let opt = coff + 20;
    let magic = rd_u16(bytes, opt)?;
    // Data directories start at +96 (PE32) / +112 (PE32+) in the
    // optional header; directory 0 is the export table.
    let dir0 = match magic {
        0x10B => opt + 96,
        0x20B => opt + 112,
        _ => {
            return Err(format!(
                "unsupported optional-header magic 0x{:04X} (not PE32/PE32+)",
                magic
            ))
        }
    };
    let export_rva = rd_u32(bytes, dir0)?;
    if export_rva == 0 {
        return Ok(Vec::new()); // no export directory at all
    }

    // Section table follows the optional header.
    let sec0 = opt + opt_size;
    let mut sections: Vec<(u32, u32, u32, u32)> = Vec::new(); // (vaddr, vsize, raw_size, raw_ptr)
    for i in 0..num_sections {
        let s = sec0 + i * 40;
        sections.push((
            rd_u32(bytes, s + 12)?,
            rd_u32(bytes, s + 8)?,
            rd_u32(bytes, s + 16)?,
            rd_u32(bytes, s + 20)?,
        ));
    }
    let rva_to_off = |rva: u32| -> Result<usize, String> {
        for &(vaddr, vsize, raw_size, raw_ptr) in &sections {
            let span = vsize.max(raw_size);
            if rva >= vaddr && (rva as u64) < vaddr as u64 + span as u64 {
                return Ok((raw_ptr as u64 + (rva - vaddr) as u64) as usize);
            }
        }
        if (rva as usize) < bytes.len() {
            // Headers are mapped 1:1 at their file offsets.
            return Ok(rva as usize);
        }
        Err(format!("RVA 0x{:X} maps to no section", rva))
    };

    let exp = rva_to_off(export_rva)?;
    let num_names = rd_u32(bytes, exp + 24)? as usize;
    let names_rva = rd_u32(bytes, exp + 32)?;
    let names_off = rva_to_off(names_rva)?;
    let mut out = Vec::with_capacity(num_names);
    for i in 0..num_names {
        let name_rva = rd_u32(bytes, names_off + i * 4)?;
        let name_off = rva_to_off(name_rva)?;
        out.push(rd_c_string(bytes, name_off)?);
    }
    Ok(out)
}

fn rd_bytes(bytes: &[u8], off: usize, len: usize) -> Result<&[u8], String> {
    bytes
        .get(off..off + len)
        .ok_or_else(|| format!("truncated PE (read of {} bytes at 0x{:X})", len, off))
}

fn rd_u16(bytes: &[u8], off: usize) -> Result<u16, String> {
    let b = rd_bytes(bytes, off, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn rd_u32(bytes: &[u8], off: usize) -> Result<u32, String> {
    let b = rd_bytes(bytes, off, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn rd_c_string(bytes: &[u8], off: usize) -> Result<String, String> {
    let tail = bytes
        .get(off..)
        .ok_or_else(|| format!("truncated PE (string at 0x{:X})", off))?;
    let end = tail
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| format!("unterminated export name at 0x{:X}", off))?;
    String::from_utf8(tail[..end].to_vec())
        .map_err(|_| format!("export name at 0x{:X} is not valid UTF-8", off))
}
