use anyhow::{anyhow, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InfoHash(pub String);

impl InfoHash {
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone)]
pub struct ParsedMagnet {
    pub infohash: InfoHash,
    pub display_name: Option<String>,
    pub trackers: Vec<String>,
}

pub fn parse(magnet: &str) -> Result<ParsedMagnet> {
    let magnet = magnet.trim();
    if !magnet.starts_with("magnet:?") {
        return Err(anyhow!("not a magnet uri"));
    }
    let query = &magnet["magnet:?".len()..];
    let mut infohash: Option<String> = None;
    let mut display_name: Option<String> = None;
    let mut trackers = Vec::new();
    for pair in query.split('&') {
        let mut it = pair.splitn(2, '=');
        let (k, v) = match (it.next(), it.next()) {
            (Some(k), Some(v)) => (k, urldecode(v)),
            _ => continue,
        };
        match k {
            "xt" => {
                if let Some(h) = v.strip_prefix("urn:btih:") {
                    infohash = Some(h.to_ascii_lowercase());
                }
            }
            "dn" => display_name = Some(v),
            "tr" => trackers.push(v),
            _ => {}
        }
    }
    let infohash = infohash.ok_or_else(|| anyhow!("magnet missing xt=urn:btih:"))?;
    if !(infohash.len() == 40 || infohash.len() == 32) {
        return Err(anyhow!("infohash wrong length: {}", infohash.len()));
    }
    Ok(ParsedMagnet { infohash: InfoHash(infohash), display_name, trackers })
}

fn urldecode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => { out.push(b' '); i += 1; }
            b'%' if i + 2 < bytes.len() => {
                let hi = hex(bytes[i+1]);
                let lo = hex(bytes[i+2]);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push(hi*16 + lo); i += 3;
                } else { out.push(bytes[i]); i += 1; }
            }
            b => { out.push(b); i += 1; }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_basic_magnet() {
        let m = parse("magnet:?xt=urn:btih:c12fe1c06bba254a9dc9f519b335aa7c1367a88a&dn=Ubuntu+ISO").unwrap();
        assert_eq!(m.infohash.as_str(), "c12fe1c06bba254a9dc9f519b335aa7c1367a88a");
        assert_eq!(m.display_name.as_deref(), Some("Ubuntu ISO"));
    }
    #[test]
    fn parses_with_trackers() {
        let m = parse("magnet:?xt=urn:btih:c12fe1c06bba254a9dc9f519b335aa7c1367a88a&tr=udp%3A%2F%2Ftracker.example%3A80").unwrap();
        assert_eq!(m.trackers, vec!["udp://tracker.example:80"]);
    }
    #[test]
    fn rejects_non_magnet() { assert!(parse("https://example.com").is_err()); }
    #[test]
    fn rejects_missing_xt() { assert!(parse("magnet:?dn=foo").is_err()); }
    #[test]
    fn rejects_wrong_length_hash() {
        assert!(parse("magnet:?xt=urn:btih:deadbeef").is_err());
    }
    #[test]
    fn infohash_lowercased() {
        let m = parse("magnet:?xt=urn:btih:C12FE1C06BBA254A9DC9F519B335AA7C1367A88A").unwrap();
        assert_eq!(m.infohash.as_str(), "c12fe1c06bba254a9dc9f519b335aa7c1367a88a");
    }
}
