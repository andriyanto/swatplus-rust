//! Plain tar archives (ustar), to hand a scenario's files to the engine and its tables back in
//! one piece where there are no files (the browser).

/// The regular files of tar archive `bytes`, by name, in order. A folder that holds every file
/// (`TxtInOut/…`) is taken off the names.
pub fn read(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    let mut long_name: Option<String> = None;
    while pos + 512 <= bytes.len() {
        let h = &bytes[pos..pos + 512];
        if h.iter().all(|&b| b == 0) {
            break;
        }
        let field = |a: usize, n: usize| -> String {
            let s = &h[a..a + n];
            let end = s.iter().position(|&b| b == 0).unwrap_or(n);
            String::from_utf8_lossy(&s[..end]).to_string()
        };
        let size_txt = field(124, 12);
        let size = usize::from_str_radix(size_txt.trim().trim_matches(char::from(0)), 8)
            .map_err(|_| format!("tar: bad size {size_txt:?} at byte {pos}"))?;
        let typ = h[156];
        let mut name = field(0, 100);
        let prefix = if &h[257..262] == b"ustar" { field(345, 155) } else { String::new() };
        if !prefix.is_empty() {
            name = format!("{prefix}/{name}");
        }
        let start = pos + 512;
        let end = start + size;
        if end > bytes.len() {
            return Err(format!("tar: {name} runs past the end"));
        }
        let data = &bytes[start..end];
        match typ {
            b'0' | 0 => {
                let name = long_name.take().unwrap_or(name);
                out.push((name, data.to_vec()));
            }
            // GNU long name: the next entry's name.
            b'L' => {
                let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
                long_name = Some(String::from_utf8_lossy(&data[..end]).to_string());
            }
            _ => {}
        }
        pos = start + size.div_ceil(512) * 512;
    }
    // One folder holding everything comes off.
    let first_dir = |n: &str| n.split_once('/').map(|(d, _)| d.to_string());
    if let Some(d) = out.first().and_then(|(n, _)| first_dir(n))
        && out.iter().all(|(n, _)| first_dir(n).as_deref() == Some(d.as_str()))
    {
        for (n, _) in out.iter_mut() {
            *n = n[d.len() + 1..].to_string();
        }
    }
    Ok(out)
}

/// A tar archive of `files` (names up to 100 bytes).
pub fn write<'a>(files: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, data) in files {
        let mut h = [0u8; 512];
        let n = name.as_bytes();
        h[..n.len().min(100)].copy_from_slice(&n[..n.len().min(100)]);
        let octal = |h: &mut [u8; 512], at: usize, width: usize, v: u64| {
            let s = format!("{:0w$o}", v, w = width - 1);
            h[at..at + width - 1].copy_from_slice(s.as_bytes());
        };
        octal(&mut h, 100, 8, 0o644);
        octal(&mut h, 108, 8, 0);
        octal(&mut h, 116, 8, 0);
        octal(&mut h, 124, 12, data.len() as u64);
        octal(&mut h, 136, 12, 0);
        h[156] = b'0';
        h[257..263].copy_from_slice(b"ustar\0");
        h[263..265].copy_from_slice(b"00");
        h[148..156].copy_from_slice(b"        ");
        let sum: u32 = h.iter().map(|&b| u32::from(b)).sum();
        let s = format!("{sum:06o}\0 ");
        h[148..156].copy_from_slice(s.as_bytes());
        out.extend_from_slice(&h);
        out.extend_from_slice(data);
        let pad = data.len().div_ceil(512) * 512 - data.len();
        out.extend(std::iter::repeat_n(0u8, pad));
    }
    out.extend(std::iter::repeat_n(0u8, 1024));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let a = b"hello".to_vec();
        let b = vec![7u8; 600];
        let t = write([("TxtInOut/file.cio", a.as_slice()), ("TxtInOut/x.txt", b.as_slice())]);
        let files = read(&t).unwrap_or_default();
        assert_eq!(files, vec![("file.cio".to_string(), a), ("x.txt".to_string(), b)]);
    }
}
