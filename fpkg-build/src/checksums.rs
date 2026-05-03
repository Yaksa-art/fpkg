use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub fn hash_bytes(data: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(data).to_hex())
}

#[allow(dead_code)]
pub fn hash_file(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub fn merkle_root(hashes: &[String]) -> String {
    if hashes.is_empty() {
        return hash_bytes(b"");
    }
    let mut sorted = hashes.to_vec();
    sorted.sort();
    let combined = sorted.join("\n");
    hash_bytes(combined.as_bytes())
}
