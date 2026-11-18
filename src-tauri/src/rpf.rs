//! Bounded, read-only RPF7 table-of-contents inspection. No extraction, decryption,
//! execution, or payload hashing. Layout reference: CodeWalker RpfFile.cs.
use crate::{model::RpfSummary, safety::relative_name};
use anyhow::{bail, Context, Result};
use std::{
    collections::HashSet,
    io::{Read, Seek, SeekFrom},
};
const MAX_TOC: u64 = 2 * 1024 * 1024;
const MAX_ENTRIES: u32 = 32768;
pub fn inspect<R: Read + Seek>(reader: &mut R, length: u64, budget: &mut u64) -> RpfSummary {
    let start = *budget;
    let mut summary = RpfSummary::default();
    let result = parse(reader, length, budget, &mut summary);
    summary.bytes_read = start - *budget;
    if let Err(e) = result {
        summary.state = "Unavailable".into();
        summary.categories.clear();
        summary.samples.clear();
        summary.notes.push(format!("RPF inspection stopped: {e}"));
    }
    summary.notes.push("Resource names are clues, not individual mod identities or proof of replacement. Payloads and nested RPF contents were not read.".into());
    summary
}
fn read<R: Read>(r: &mut R, out: &mut [u8], budget: &mut u64) -> Result<()> {
    if out.len() as u64 > *budget {
        bail!("scan read budget reached");
    }
    *budget -= out.len() as u64;
    r.read_exact(out)?;
    Ok(())
}
fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn parse<R: Read + Seek>(
    r: &mut R,
    length: u64,
    budget: &mut u64,
    out: &mut RpfSummary,
) -> Result<()> {
    if length < 16 {
        bail!("truncated RPF header");
    }
    r.seek(SeekFrom::Start(0))?;
    let mut header = [0u8; 16];
    read(r, &mut header, budget)?;
    if word(&header, 0) != 0x52504637 {
        bail!("not a supported RPF7 archive");
    }
    let count = word(&header, 4);
    let names = word(&header, 8);
    let encryption = word(&header, 12);
    out.entries = count;
    if count == 0 || count > MAX_ENTRIES {
        bail!("entry count exceeds safe limits");
    }
    let table = count as u64 * 16;
    let size = table
        .checked_add(names as u64)
        .context("TOC size overflow")?;
    if size > MAX_TOC || names == 0 || size + 16 > length {
        bail!("oversized or truncated RPF table");
    }
    if encryption != 0 && encryption != 0x4e45504f {
        out.state = "Encrypted".into();
        out.notes
            .push("Encrypted or unknown TOC; no game keys are requested or extracted.".into());
        return Ok(());
    }
    let mut data = vec![0u8; size as usize];
    read(r, &mut data, budget)?;
    let table_data = &data[..table as usize];
    let names_data = &data[table as usize..];
    let mut entries = Vec::new();
    for entry in table_data.chunks_exact(16) {
        let directory = word(entry, 4) == 0x7fffff00;
        let offset = if directory {
            word(entry, 0)
        } else {
            word(entry, 0) & 0xffff
        } as usize;
        let name_data = names_data
            .get(offset..)
            .context("name offset outside table")?;
        let end = name_data
            .iter()
            .take(257)
            .position(|b| *b == 0)
            .context("unterminated or oversized RPF name")?;
        let name = std::str::from_utf8(&name_data[..end])
            .context("invalid RPF filename encoding")?
            .to_string();
        if !name.is_empty() {
            relative_name(&name)?;
            if name.contains(['/', '\\']) {
                bail!("RPF entry name contains separators");
            }
        }
        entries.push((name, directory, word(entry, 8), word(entry, 12)));
    }
    if !entries[0].1 {
        bail!("RPF root is not a directory");
    }
    let mut stack = vec![(0usize, String::new(), 0usize)];
    let mut visited = HashSet::new();
    let mut discovered = HashSet::from([0usize]);
    let mut paths = HashSet::new();
    while let Some((index, parent, depth)) = stack.pop() {
        if depth > 32 || !visited.insert(index) {
            bail!("cyclic or excessively deep RPF directory");
        }
        let (_, _, first, count) = &entries[index];
        let end = (*first as usize)
            .checked_add(*count as usize)
            .context("directory range overflow")?;
        if end > entries.len() {
            bail!("RPF directory range outside table");
        }
        for i in *first as usize..end {
            if !discovered.insert(i) {
                bail!("RPF entry has multiple parents or a directory cycle");
            }
            if i == 0 {
                bail!("cyclic RPF directory");
            }
            let (name, dir, _, _) = &entries[i];
            if name.is_empty() {
                bail!("empty RPF child name");
            }
            let path = if parent.is_empty() {
                name.clone()
            } else {
                format!("{parent}/{name}")
            };
            if path.len() > 1024 || !paths.insert(path.to_lowercase()) {
                bail!("duplicate or oversized RPF path");
            }
            if *dir {
                stack.push((i, path, depth + 1));
            } else {
                if !visited.insert(i) {
                    bail!("multiply referenced RPF file");
                }
                let category = category_hint(&path);
                if let Some(category) = category {
                    *out.categories.entry(category.into()).or_default() += 1;
                }
                if out.samples.len() < 80 {
                    out.samples.push(path);
                }
            }
        }
    }
    if visited.len() != entries.len() {
        bail!("unreferenced RPF entries");
    }
    out.state = "Readable TOC".into();
    Ok(())
}
pub fn category_hint(name: &str) -> Option<&'static str> {
    let n = name.to_lowercase();
    if n.contains("weapon") || n.rsplit('/').next().unwrap_or("").starts_with("w_") {
        Some("Weapons")
    } else if n.contains("cloth")
        || n.contains("mp_m_")
        || n.contains("mp_f_")
        || n.contains("uppr_")
        || n.contains("lowr_")
    {
        Some("Characters")
    } else if n.ends_with(".yft") || n.contains("vehicle") || n.contains("handling.meta") {
        Some("Vehicles")
    } else if n.ends_with(".ymap") || n.ends_with(".ytyp") {
        Some("Maps")
    } else {
        None
    }
}
