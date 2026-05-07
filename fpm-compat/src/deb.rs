use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead, BufReader, Read},
    path::Path,
};
use tracing::debug;
use crate::{CompatError, convert::{ForeignDep, ForeignPackage}};

pub fn parse(path: &Path) -> Result<ForeignPackage, CompatError> {
    let f = File::open(path)?;
    let mut archive = ar::Archive::new(f);
    let control_gz = extract_control_tar(&mut archive)?;
    let fields = parse_control_bytes(&control_gz)?;
    build_package(fields)
}

fn extract_control_tar(archive: &mut ar::Archive<File>) -> Result<Vec<u8>, CompatError> {
    while let Some(entry) = archive.next_entry() {
        let mut entry = entry.map_err(|e| CompatError::Archive(e.to_string()))?;
        let name = String::from_utf8_lossy(entry.header().identifier()).to_string();
        if name.starts_with("control.tar") {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            let control_content = decompress_tar_member(&name, &buf, "./control")?;
            return Ok(control_content);
        }
    }
    Err(CompatError::Archive("control.tar not found in .deb".into()))
}

fn decompress_tar_member(name: &str, data: &[u8], member: &str) -> Result<Vec<u8>, CompatError> {
    use std::io::Cursor;

    let cursor = Cursor::new(data);
    let tar_data: Box<dyn Read> = if name.ends_with(".zst") {
        Box::new(zstd::stream::read::Decoder::new(cursor)
            .map_err(|e| CompatError::Archive(e.to_string()))?)
    } else if name.ends_with(".gz") {
        Box::new(flate2::read::GzDecoder::new(cursor))
    } else if name.ends_with(".bz2") {
        Box::new(bzip2::read::BzDecoder::new(cursor))
    } else {
        Box::new(cursor)
    };

    let mut tar = tar::Archive::new(tar_data);
    for entry in tar.entries().map_err(|e| CompatError::Archive(e.to_string()))? {
        let mut entry = entry.map_err(|e| CompatError::Archive(e.to_string()))?;
        let entry_path = entry.path().map_err(|e| CompatError::Archive(e.to_string()))?;
        if entry_path.to_str().map_or(false, |p| p == member || p == &member[2..]) {
            let mut out = Vec::new();
            entry.read_to_end(&mut out)?;
            return Ok(out);
        }
    }
    Err(CompatError::Archive(format!("member '{}' not found in control.tar", member)))
}

fn parse_control_bytes(data: &[u8]) -> Result<HashMap<String, String>, CompatError> {
    let mut map = HashMap::new();
    let mut cur_key = String::new();
    let mut cur_val = String::new();

    for line in BufReader::new(data).lines() {
        let line = line?;
        if line.starts_with(' ') || line.starts_with('\t') {
            cur_val.push('\n');
            cur_val.push_str(line.trim_start());
        } else if let Some(colon) = line.find(':') {
            if !cur_key.is_empty() {
                map.insert(cur_key.clone(), cur_val.trim().to_string());
            }
            cur_key = line[..colon].trim().to_lowercase();
            cur_val = line[colon + 1..].trim().to_string();
        }
    }
    if !cur_key.is_empty() {
        map.insert(cur_key, cur_val.trim().to_string());
    }
    Ok(map)
}

fn build_package(f: HashMap<String, String>) -> Result<ForeignPackage, CompatError> {
    let get = |k: &str| -> Result<String, CompatError> {
        f.get(k).cloned().ok_or_else(|| CompatError::MissingField(k.into()))
    };

    let depends = f.get("depends")
        .map(|s| parse_deb_deps(s))
        .unwrap_or_default();

    let conflicts = f.get("conflicts")
        .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
        .unwrap_or_default();

    let provides = f.get("provides")
        .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
        .unwrap_or_default();

    let installed_size = f.get("installed-size")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0) * 1024;

    debug!(name = %f.get("package").unwrap_or(&String::new()), "parsed .deb control");

    Ok(ForeignPackage {
        format:       "deb".into(),
        name:         get("package")?,
        version:      get("version")?,
        release:      "1".into(),
        arch:         deb_arch(f.get("architecture").map(String::as_str).unwrap_or("amd64")),
        summary:      f.get("description")
                        .and_then(|d| d.lines().next())
                        .unwrap_or("").to_string(),
        description:  get("description").unwrap_or_default(),
        license:      f.get("license").cloned().unwrap_or_else(|| "unknown".into()),
        homepage:     f.get("homepage").cloned().unwrap_or_default(),
        maintainer:   f.get("maintainer").cloned().unwrap_or_default(),
        depends,
        conflicts,
        provides,
        installed_size,
    })
}

fn parse_deb_deps(s: &str) -> Vec<ForeignDep> {
    s.split(',').map(|chunk| {
        let chunk = chunk.trim();
        let (name, version) = if let Some(p) = chunk.find('(') {
            let n = chunk[..p].trim().to_string();
            let v = chunk[p+1..]
                .trim_end_matches(')')
                .trim()
                .trim_start_matches(|c: char| !c.is_alphanumeric())
                .to_string();
            (n, Some(v))
        } else {
            (chunk.to_string(), None)
        };
        ForeignDep { name, version, optional: false }
    }).filter(|d| !d.name.is_empty()).collect()
}

fn deb_arch(a: &str) -> String {
    match a {
        "amd64"   => "x86_64",
        "arm64"   => "aarch64",
        "i386"    => "i686",
        "all"     => "any",
        other     => other,
    }.to_string()
}
