use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;

use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use crate::checksums::{hash_bytes, merkle_root};
use crate::manifest::Manifest;

pub struct DataFile {
    pub archive_path: String,
    pub content: Vec<u8>,
}

pub struct FpkgWriter {
    pub output_path: String,
    pub manifest: Manifest,
    pub data_files: Vec<DataFile>,
    pub scripts: Vec<(String, Vec<u8>)>,
    pub changelog: String,
    pub origin: String,
}

impl FpkgWriter {
    pub fn new(output_path: String, manifest: Manifest) -> Self {
        Self {
            output_path,
            manifest,
            data_files: Vec::new(),
            scripts: Vec::new(),
            changelog: String::new(),
            origin: "native".into(),
        }
    }

    pub fn add_data_file(&mut self, rel_path: &str, content: Vec<u8>) {
        let archive_path = if rel_path.starts_with("DATA/") {
            rel_path.to_string()
        } else {
            format!("DATA/{}", rel_path.trim_start_matches('/'))
        };
        self.data_files.push(DataFile { archive_path, content });
    }

    pub fn add_script(&mut self, name: &str, content: Vec<u8>) {
        self.scripts.push((name.to_string(), content));
    }

    fn build_checksums(&self) -> String {
        self.data_files
            .iter()
            .map(|f| format!("{}  {}", hash_bytes(&f.content), f.archive_path))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn compute_merkle(&self) -> String {
        let hashes: Vec<String> = self.data_files
            .iter()
            .map(|f| hash_bytes(&f.content))
            .collect();
        merkle_root(&hashes)
    }

    fn build_deps_toml(&self) -> String {
        let deps = &self.manifest.dependencies;
        let mut lines = Vec::new();

        for req in &deps.requires {
            let (name, version) = split_dep(req);
            lines.push(format!(
                "[[dep]]\nname     = {:?}\nversion  = {:?}\noptional = false\nreason   = \"\"\n",
                name, version
            ));
        }
        for sug in &deps.suggests {
            let (name, version) = split_dep(sug);
            lines.push(format!(
                "[[dep]]\nname     = {:?}\nversion  = {:?}\noptional = true\ngroup    = \"optional\"\nreason   = \"\"\n",
                name, version
            ));
        }
        lines.join("\n")
    }

    pub fn write(mut self) -> Result<u64> {
        let checksums = self.build_checksums();
        let merkle = self.compute_merkle();
        self.manifest.verification.content_tree = merkle;

        let manifest_toml = self.manifest.to_toml()?;
        let manifest_hash = hash_bytes(manifest_toml.as_bytes());
        self.manifest.verification.manifest_hash = manifest_hash;

        let manifest_toml = self.manifest.to_toml()?;

        let _installed_size: u64 = self.data_files.iter().map(|f| f.content.len() as u64).sum();

        let file = File::create(&self.output_path)
            .with_context(|| format!("Cannot create {}", self.output_path))?;
        let mut zip = ZipWriter::new(file);

        let opts: FileOptions = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(9));

        zip.start_file("META/manifest.toml", opts)?;
        zip.write_all(manifest_toml.as_bytes())?;

        zip.start_file("META/checksums.blake3", opts)?;
        zip.write_all(checksums.as_bytes())?;

        let deps_toml = self.build_deps_toml();
        if !deps_toml.is_empty() {
            zip.start_file("META/dependencies.toml", opts)?;
            zip.write_all(deps_toml.as_bytes())?;
        }

        if !self.changelog.is_empty() {
            zip.start_file("META/changelog.md", opts)?;
            zip.write_all(self.changelog.as_bytes())?;
        }

        for (name, content) in &self.scripts {
            zip.start_file(format!("META/scripts/{}", name), opts)?;
            zip.write_all(content)?;
        }

        for file in &self.data_files {
            zip.start_file(&file.archive_path, opts)?;
            zip.write_all(&file.content)?;
        }

        zip.start_file("COMPAT/origin_format.txt", opts)?;
        zip.write_all(self.origin.as_bytes())?;

        zip.finish()?;

        let compressed = std::fs::metadata(&self.output_path)?.len();
        Ok(compressed)
    }
}

fn split_dep(dep: &str) -> (&str, &str) {
    match dep.find(|c: char| c == '>' || c == '<' || c == '=' || c == '^' || c == '~') {
        Some(idx) => {
            let name = dep[..idx].trim();
            let ver = dep[idx..].trim();
            (name, ver)
        }
        None => (dep.trim(), ""),
    }
}
