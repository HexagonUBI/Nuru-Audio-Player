use std::collections::HashMap;
use std::path::Path;

const ORIGIN: &str = "http://tauri.localhost";
const BLOCK: usize = 32 * 1024;
const TABLE_MAGIC: u64 = 0xdb47_7524_8b80_fb57;

struct Entry {
    seq: u64,
    deleted: bool,
    key: Vec<u8>,
    value: Vec<u8>,
}

pub fn read_local_storage(dir: &Path) -> HashMap<String, String> {
    let mut entries = Vec::new();
    let Ok(listing) = std::fs::read_dir(dir) else {
        return HashMap::new();
    };
    for file in listing.flatten() {
        let path = file.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let Ok(bytes) = std::fs::read(&path) else { continue };
        match ext {
            "log" => read_log(&bytes, &mut entries),
            "ldb" | "sst" => {
                if read_table(&bytes, &mut entries).is_none() {
                    log::warn!("skipped unreadable storage table {}", path.display());
                }
            }
            _ => {}
        }
    }
    newest_values(entries)
}

fn newest_values(entries: Vec<Entry>) -> HashMap<String, String> {
    let mut prefix = vec![b'_'];
    prefix.extend_from_slice(ORIGIN.as_bytes());
    prefix.push(0);

    let mut latest: HashMap<Vec<u8>, Entry> = HashMap::new();
    for entry in entries {
        if !entry.key.starts_with(&prefix) {
            continue;
        }
        let newer = latest.get(&entry.key).is_none_or(|seen| entry.seq >= seen.seq);
        if newer {
            latest.insert(entry.key.clone(), entry);
        }
    }

    latest
        .into_values()
        .filter(|e| !e.deleted)
        .filter_map(|e| Some((decode(&e.key[prefix.len()..])?, decode(&e.value)?)))
        .collect()
}

fn decode(bytes: &[u8]) -> Option<String> {
    let (&kind, rest) = bytes.split_first()?;
    match kind {
        1 => Some(rest.iter().map(|&b| b as char).collect()),
        0 => {
            let units: Vec<u16> = rest.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            String::from_utf16(&units).ok()
        }
        _ => None,
    }
}

fn read_log(bytes: &[u8], out: &mut Vec<Entry>) {
    let mut record = Vec::new();
    let mut pos = 0;
    while pos + 7 <= bytes.len() {
        let left = BLOCK - pos % BLOCK;
        if left < 7 {
            pos += left;
            continue;
        }
        let len = u16::from_le_bytes([bytes[pos + 4], bytes[pos + 5]]) as usize;
        let kind = bytes[pos + 6];
        let start = pos + 7;
        let end = start + len;
        if end > bytes.len() {
            break;
        }
        let payload = &bytes[start..end];
        match kind {
            1 => parse_batch(payload, out),
            2 => {
                record.clear();
                record.extend_from_slice(payload);
            }
            3 => record.extend_from_slice(payload),
            4 => {
                record.extend_from_slice(payload);
                parse_batch(&record, out);
                record.clear();
            }
            _ => {}
        }
        pos = end;
    }
}

fn parse_batch(batch: &[u8], out: &mut Vec<Entry>) {
    if batch.len() < 12 {
        return;
    }
    let seq = u64::from_le_bytes(batch[0..8].try_into().unwrap_or_default());
    let count = u32::from_le_bytes(batch[8..12].try_into().unwrap_or_default()) as u64;
    let mut p = 12;
    for i in 0..count {
        let Some(&tag) = batch.get(p) else { return };
        p += 1;
        let Some(key) = slice(batch, &mut p) else { return };
        match tag {
            1 => {
                let Some(value) = slice(batch, &mut p) else { return };
                out.push(Entry { seq: seq + i, deleted: false, key: key.to_vec(), value: value.to_vec() });
            }
            0 => out.push(Entry { seq: seq + i, deleted: true, key: key.to_vec(), value: Vec::new() }),
            _ => return,
        }
    }
}

fn read_table(bytes: &[u8], out: &mut Vec<Entry>) -> Option<()> {
    let footer = bytes.get(bytes.len().checked_sub(48)?..)?;
    if u64::from_le_bytes(footer[40..48].try_into().ok()?) != TABLE_MAGIC {
        return None;
    }
    let mut p = 0;
    varint(footer, &mut p)?;
    varint(footer, &mut p)?;
    let index_offset = varint(footer, &mut p)?;
    let index_size = varint(footer, &mut p)?;

    let index = read_block(bytes, index_offset, index_size)?;
    for (_, handle) in block_entries(&index) {
        let mut q = 0;
        let offset = varint(&handle, &mut q)?;
        let size = varint(&handle, &mut q)?;
        let data = read_block(bytes, offset, size)?;
        for (internal, value) in block_entries(&data) {
            if internal.len() < 8 {
                continue;
            }
            let (key, tail) = internal.split_at(internal.len() - 8);
            let packed = u64::from_le_bytes(tail.try_into().ok()?);
            out.push(Entry { seq: packed >> 8, deleted: packed & 0xff == 0, key: key.to_vec(), value });
        }
    }
    Some(())
}

fn read_block(bytes: &[u8], offset: u64, size: u64) -> Option<Vec<u8>> {
    let start = usize::try_from(offset).ok()?;
    let end = start.checked_add(usize::try_from(size).ok()?)?;
    let raw = bytes.get(start..end)?;
    match bytes.get(end)? {
        0 => Some(raw.to_vec()),
        1 => snap::raw::Decoder::new().decompress_vec(raw).ok(),
        _ => None,
    }
}

fn block_entries(block: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut out = Vec::new();
    let Some(count_at) = block.len().checked_sub(4) else { return out };
    let restarts = u32::from_le_bytes(block[count_at..].try_into().unwrap_or_default()) as usize;
    let Some(end) = count_at.checked_sub(restarts * 4) else { return out };

    let mut p = 0;
    let mut key: Vec<u8> = Vec::new();
    while p < end {
        let (Some(shared), Some(fresh), Some(vlen)) = (varint(block, &mut p), varint(block, &mut p), varint(block, &mut p)) else {
            break;
        };
        let (shared, fresh, vlen) = (shared as usize, fresh as usize, vlen as usize);
        if shared > key.len() || p + fresh + vlen > end {
            break;
        }
        key.truncate(shared);
        key.extend_from_slice(&block[p..p + fresh]);
        p += fresh;
        out.push((key.clone(), block[p..p + vlen].to_vec()));
        p += vlen;
    }
    out
}

fn slice<'a>(bytes: &'a [u8], p: &mut usize) -> Option<&'a [u8]> {
    let len = varint(bytes, p)? as usize;
    let s = bytes.get(*p..p.checked_add(len)?)?;
    *p += len;
    Some(s)
}

fn varint(bytes: &[u8], p: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let b = *bytes.get(*p)?;
        *p += 1;
        value |= u64::from(b & 0x7f) << shift;
        if b & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn varint_bytes(mut v: u64, out: &mut Vec<u8>) {
        while v >= 0x80 {
            out.push((v as u8) | 0x80);
            v >>= 7;
        }
        out.push(v as u8);
    }

    fn key(name: &str) -> Vec<u8> {
        let mut k = vec![b'_'];
        k.extend_from_slice(ORIGIN.as_bytes());
        k.push(0);
        k.push(1);
        k.extend_from_slice(name.as_bytes());
        k
    }

    fn latin(text: &str) -> Vec<u8> {
        let mut v = vec![1];
        v.extend_from_slice(text.as_bytes());
        v
    }

    fn batch(seq: u64, puts: &[(Vec<u8>, Option<Vec<u8>>)]) -> Vec<u8> {
        let mut b = seq.to_le_bytes().to_vec();
        b.extend_from_slice(&(puts.len() as u32).to_le_bytes());
        for (k, v) in puts {
            b.push(if v.is_some() { 1 } else { 0 });
            varint_bytes(k.len() as u64, &mut b);
            b.extend_from_slice(k);
            if let Some(v) = v {
                varint_bytes(v.len() as u64, &mut b);
                b.extend_from_slice(v);
            }
        }
        b
    }

    fn log_file(batches: &[Vec<u8>]) -> Vec<u8> {
        let mut out = Vec::new();
        for b in batches {
            out.extend_from_slice(&[0, 0, 0, 0]);
            out.extend_from_slice(&(b.len() as u16).to_le_bytes());
            out.push(1);
            out.extend_from_slice(b);
        }
        out
    }

    #[test]
    fn newest_write_wins_and_deletes_hide() {
        let bytes = log_file(&[
            batch(1, &[(key("nuru.state.v1"), Some(latin("{\"a\":1}"))), (key("gone"), Some(latin("x")))]),
            batch(5, &[(key("nuru.state.v1"), Some(latin("{\"a\":2}"))), (key("gone"), None)]),
        ]);
        let mut entries = Vec::new();
        read_log(&bytes, &mut entries);
        let values = newest_values(entries);
        assert_eq!(values.get("nuru.state.v1").map(String::as_str), Some("{\"a\":2}"));
        assert!(!values.contains_key("gone"));
    }

    #[test]
    fn other_origins_are_ignored_and_utf16_decodes() {
        let mut dev = b"_http://127.0.0.1:5183\x00\x01nuru.state.v1".to_vec();
        dev.shrink_to_fit();
        let utf16: Vec<u8> = std::iter::once(0u8)
            .chain("{\"n\":\"\u{e9}\"}".encode_utf16().flat_map(|u| u.to_le_bytes()))
            .collect();
        let bytes = log_file(&[batch(1, &[(dev, Some(latin("dev"))), (key("nuru.presets.v1"), Some(utf16))])]);
        let mut entries = Vec::new();
        read_log(&bytes, &mut entries);
        let values = newest_values(entries);
        assert_eq!(values.len(), 1);
        assert_eq!(values.get("nuru.presets.v1").map(String::as_str), Some("{\"n\":\"\u{e9}\"}"));
    }
}

#[cfg(test)]
mod real {
    #[test]
    #[ignore]
    fn summarise_a_real_storage_copy() {
        let Some(dir) = std::env::var_os("NURU_LEGACY_TEST") else { return };
        let values = super::read_local_storage(std::path::Path::new(&dir));
        let mut keys: Vec<_> = values.keys().cloned().collect();
        keys.sort();
        println!("keys: {keys:?}");
        if let Some(raw) = values.get("nuru.state.v1") {
            let saved: crate::saved::Saved = serde_json::from_str(raw).expect("state parses");
            println!("state: {} layer(s), theme {}, tile {:?}", saved.layers.len(), saved.theme, saved.tile_size);
        }
        if let Some(raw) = values.get("nuru.presets.v1") {
            let presets: crate::saved::PresetFile = serde_json::from_str(raw).expect("presets parse");
            println!("presets: {}", presets.items.len());
        }
        if let Some(raw) = values.get("nuru.schedule.v1") {
            let slots = serde_json::from_str::<crate::saved::ScheduleFile>(raw).expect("schedule parses").normalised();
            println!("schedule: {} slot(s) set", slots.iter().filter(|s| s.is_some()).count());
        }
    }
}
