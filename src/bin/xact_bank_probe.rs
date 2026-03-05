use std::env;
use std::fs;
use std::path::Path;

const SEGMENT_COUNT: usize = 5;

#[derive(Clone, Copy, Debug)]
enum Endianness {
    Little,
    Big,
}

#[derive(Clone, Copy, Debug)]
struct Segment {
    offset: u32,
    length: u32,
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(path_arg) = args.next() else {
        eprintln!("usage: cargo run --bin xact_bank_probe -- <path-to-bank>");
        std::process::exit(1);
    };

    let path = Path::new(&path_arg);
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to read {}: {error}", path.display());
            std::process::exit(1);
        }
    };

    println!("file: {}", path.display());
    println!("size: {} bytes", bytes.len());

    if bytes.len() < 12 {
        println!("file too small for bank header");
        return;
    }

    match bytes.get(0..4) {
        Some(b"DNBW") | Some(b"WBND") => probe_xwb(&bytes),
        Some(sig) => {
            let ascii_sig = sig
                .iter()
                .map(|byte| {
                    if byte.is_ascii_graphic() {
                        char::from(*byte)
                    } else {
                        '.'
                    }
                })
                .collect::<String>();
            println!("signature: {}", ascii_sig);
            probe_printable_strings(&bytes, 6);
        }
        None => {
            println!("unable to read signature");
        }
    }
}

fn probe_xwb(bytes: &[u8]) {
    let signature = bytes.get(0..4).unwrap_or_default();
    let sig = std::str::from_utf8(signature).unwrap_or("????");
    let endianness = if signature == b"WBND" {
        Endianness::Little
    } else {
        Endianness::Big
    };

    println!("signature: {sig}");
    println!("endianness: {endianness:?}");

    let version = read_u32(bytes, 4, endianness).unwrap_or(0);
    let header_version = read_u32(bytes, 8, endianness).unwrap_or(0);
    println!("version: {version}");
    println!("header_version: {header_version}");

    let mut segments = [Segment {
        offset: 0,
        length: 0,
    }; SEGMENT_COUNT];

    for (index, segment) in segments.iter_mut().enumerate() {
        let base = 12 + (index * 8);
        let offset = read_u32(bytes, base, endianness).unwrap_or(0);
        let length = read_u32(bytes, base + 4, endianness).unwrap_or(0);
        *segment = Segment { offset, length };
        println!("segment[{index}] offset=0x{offset:08X} length=0x{length:08X} ({length})");
    }

    let Some(bank_segment) = segments.first().copied() else {
        println!("missing bank data segment");
        return;
    };

    let bank_slice = slice_region(bytes, bank_segment.offset, bank_segment.length);
    let Some(bank_slice) = bank_slice else {
        println!("bank data segment is out of bounds");
        return;
    };

    println!("bank_data_bytes: {}", bank_slice.len());
    println!("bank_data_hex[0..64]:");
    dump_hex_prefix(bank_slice, 64);

    let fixed_name = read_c_string(bank_slice.get(8..72).unwrap_or_default());
    if !fixed_name.is_empty() {
        println!("bank_name_fixed: {fixed_name}");
    }

    if let Some(len_byte) = bank_slice.get(8).copied()
        && len_byte > 0
        && len_byte < 64
    {
        let start = 9usize;
        let end = start + usize::from(len_byte);
        if end <= bank_slice.len() {
            let name = read_c_string(&bank_slice[start..end]);
            if !name.is_empty() {
                println!("bank_name_len_prefixed: {name}");
            }
        }
    }

    let entry_segment = segments.get(1).copied().unwrap_or(Segment {
        offset: 0,
        length: 0,
    });
    let entry_count = read_u32(bank_slice, 4, endianness).unwrap_or(0);
    let entry_meta_size = read_u32(bank_slice, 72, endianness).unwrap_or(0);
    println!("entry_count: {entry_count}");
    println!("entry_meta_size: {entry_meta_size}");
    println!("entry_meta_len: {}", entry_segment.length);

    let candidates = guess_entry_layout(bank_slice, entry_segment.length, endianness);
    if candidates.is_empty() {
        println!("entry layout candidates: none (needs manual decode)");
    } else {
        println!("entry layout candidates:");
        for (entry_count_offs, entry_count, entry_size_offs, entry_size) in candidates {
            println!(
                "  entry_count@{entry_count_offs}={entry_count}, entry_size@{entry_size_offs}={entry_size}"
            );
        }
    }

    probe_entry_formats(
        bytes,
        entry_segment,
        endianness,
        entry_count,
        entry_meta_size,
    );

    probe_printable_strings(bank_slice, 6);
}

fn probe_entry_formats(
    bytes: &[u8],
    entry_segment: Segment,
    endianness: Endianness,
    entry_count: u32,
    entry_meta_size: u32,
) {
    if entry_count == 0 || entry_meta_size == 0 {
        return;
    }

    let Some(entry_slice) = slice_region(bytes, entry_segment.offset, entry_segment.length) else {
        return;
    };

    let entry_size = usize::try_from(entry_meta_size)
        .ok()
        .filter(|size| *size >= 8);
    let Some(entry_size) = entry_size else {
        return;
    };

    let max_entries = usize::try_from(entry_count).unwrap_or(0);
    let available_entries = entry_slice.len() / entry_size;
    let inspect_entries = available_entries.min(max_entries).min(6);

    if inspect_entries == 0 {
        return;
    }

    println!("entry format samples:");
    for index in 0..inspect_entries {
        let base = index * entry_size;
        let duration_flags = read_u32(entry_slice, base, endianness).unwrap_or(0);
        let raw = read_u32(entry_slice, base + 4, endianness).unwrap_or(0);
        let raw_swapped = raw.swap_bytes();

        let lo = decode_miniwave_low(raw);
        let hi = decode_miniwave_high(raw);
        let lo_swapped = decode_miniwave_low(raw_swapped);
        let hi_swapped = decode_miniwave_high(raw_swapped);

        println!(
            "  #{index}: duration_flags=0x{duration_flags:08X} raw=0x{raw:08X} raw_swapped=0x{raw_swapped:08X}",
        );
        println!(
            "       low(tag={},ch={},hz={},align={},bits={}) high(tag={},ch={},hz={},align={},bits={})",
            lo.format_tag,
            lo.channels,
            lo.samples_per_sec,
            lo.block_align,
            lo.bits_per_sample,
            hi.format_tag,
            hi.channels,
            hi.samples_per_sec,
            hi.block_align,
            hi.bits_per_sample,
        );
        println!(
            "       low_swapped(tag={},ch={},hz={},align={},bits={}) high_swapped(tag={},ch={},hz={},align={},bits={})",
            lo_swapped.format_tag,
            lo_swapped.channels,
            lo_swapped.samples_per_sec,
            lo_swapped.block_align,
            lo_swapped.bits_per_sample,
            hi_swapped.format_tag,
            hi_swapped.channels,
            hi_swapped.samples_per_sec,
            hi_swapped.block_align,
            hi_swapped.bits_per_sample,
        );
    }
}

#[derive(Clone, Copy)]
struct MiniWaveDecode {
    format_tag: u32,
    channels: u32,
    samples_per_sec: u32,
    block_align: u32,
    bits_per_sample: u32,
}

fn decode_miniwave_low(raw: u32) -> MiniWaveDecode {
    MiniWaveDecode {
        format_tag: raw & 0x3,
        channels: (raw >> 2) & 0x7,
        samples_per_sec: (raw >> 5) & 0x3_FFFF,
        block_align: (raw >> 23) & 0xFF,
        bits_per_sample: (raw >> 31) & 0x1,
    }
}

fn decode_miniwave_high(raw: u32) -> MiniWaveDecode {
    MiniWaveDecode {
        format_tag: (raw >> 30) & 0x3,
        channels: (raw >> 27) & 0x7,
        samples_per_sec: (raw >> 9) & 0x3_FFFF,
        block_align: (raw >> 1) & 0xFF,
        bits_per_sample: raw & 0x1,
    }
}

fn guess_entry_layout(
    bank_slice: &[u8],
    entry_meta_len: u32,
    endianness: Endianness,
) -> Vec<(usize, u32, usize, u32)> {
    let mut candidates = Vec::new();

    for entry_count_offs in (0..48).step_by(4) {
        let Some(entry_count) = read_u32(bank_slice, entry_count_offs, endianness) else {
            continue;
        };
        if entry_count == 0 || entry_count > 10_000 {
            continue;
        }

        for entry_size_offs in (0..64).step_by(4) {
            let Some(entry_size) = read_u32(bank_slice, entry_size_offs, endianness) else {
                continue;
            };
            if entry_size == 0 || entry_size > 512 {
                continue;
            }

            let expected = entry_count.saturating_mul(entry_size);
            if expected == entry_meta_len {
                candidates.push((entry_count_offs, entry_count, entry_size_offs, entry_size));
            }
        }
    }

    candidates
}

fn probe_printable_strings(bytes: &[u8], min_len: usize) {
    println!("printable strings (min {min_len} chars):");

    let mut start = 0usize;
    let mut in_run = false;
    for (index, byte) in bytes.iter().enumerate() {
        let printable = byte.is_ascii_graphic() || *byte == b' ';
        if printable {
            if !in_run {
                start = index;
                in_run = true;
            }
            continue;
        }

        if in_run {
            let run = &bytes[start..index];
            if run.len() >= min_len {
                let value = String::from_utf8_lossy(run);
                println!("  0x{start:08X}: {value}");
            }
            in_run = false;
        }
    }

    if in_run {
        let run = &bytes[start..];
        if run.len() >= min_len {
            let value = String::from_utf8_lossy(run);
            println!("  0x{start:08X}: {value}");
        }
    }
}

fn dump_hex_prefix(bytes: &[u8], max_len: usize) {
    let len = bytes.len().min(max_len);
    let mut index = 0usize;
    while index < len {
        let end = (index + 16).min(len);
        let chunk = &bytes[index..end];
        let hex = chunk
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!("  0x{index:04X}: {hex}");
        index = end;
    }
}

fn read_u32(bytes: &[u8], offset: usize, endianness: Endianness) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    let value = match endianness {
        Endianness::Little => u32::from_le_bytes(slice.try_into().ok()?),
        Endianness::Big => u32::from_be_bytes(slice.try_into().ok()?),
    };
    Some(value)
}

fn slice_region(bytes: &[u8], offset: u32, length: u32) -> Option<&[u8]> {
    let start = usize::try_from(offset).ok()?;
    let len = usize::try_from(length).ok()?;
    let end = start.checked_add(len)?;
    bytes.get(start..end)
}

fn read_c_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let slice = &bytes[..end];
    let printable = slice
        .iter()
        .all(|byte| byte.is_ascii_graphic() || *byte == b' ');
    if !printable {
        return String::new();
    }

    String::from_utf8_lossy(slice).trim().to_string()
}
