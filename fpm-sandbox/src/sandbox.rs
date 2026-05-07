use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::SandboxError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxLevel {
    None    = 0,
    Overlay = 1,
    Bubble  = 2,
    Full    = 3,
}

impl std::fmt::Display for SandboxLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxLevel::None    => write!(f, "none"),
            SandboxLevel::Overlay => write!(f, "overlay"),
            SandboxLevel::Bubble  => write!(f, "bubble"),
            SandboxLevel::Full    => write!(f, "full"),
        }
    }
}

impl std::str::FromStr for SandboxLevel {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" | "none"    => Ok(SandboxLevel::None),
            "1" | "overlay" => Ok(SandboxLevel::Overlay),
            "2" | "bubble"  => Ok(SandboxLevel::Bubble),
            "3" | "full"    => Ok(SandboxLevel::Full),
            other => Err(anyhow::anyhow!("unknown sandbox level: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub level:      SandboxLevel,
    pub pkg_name:   String,
    pub base_dir:   PathBuf,
    pub overlay_dir: PathBuf,
    pub network:    bool,
    pub read_only_paths: Vec<PathBuf>,
    pub bind_paths:      Vec<(PathBuf, PathBuf)>,
}

impl SandboxConfig {
    pub fn new(pkg_name: impl Into<String>, level: SandboxLevel) -> Self {
        let pkg_name = pkg_name.into();
        let base = dirs_base(&pkg_name);
        let overlay = base.join("overlay");
        SandboxConfig {
            level,
            pkg_name,
            base_dir: base,
            overlay_dir: overlay,
            network: false,
            read_only_paths: vec![],
            bind_paths: vec![],
        }
    }

    pub fn upper_dir(&self) -> PathBuf  { self.overlay_dir.join("upper") }
    pub fn work_dir(&self)  -> PathBuf  { self.overlay_dir.join("work") }
    pub fn merge_dir(&self) -> PathBuf  { self.overlay_dir.join("merge") }
    pub fn lower_dir(&self) -> PathBuf  { self.overlay_dir.join("lower") }
}

pub trait Sandbox {
    fn enter(&self, cfg: &SandboxConfig) -> Result<(), SandboxError>;
    fn leave(&self, cfg: &SandboxConfig) -> Result<(), SandboxError>;
    fn run(&self, cfg: &SandboxConfig, argv: &[&str]) -> Result<i32, SandboxError>;
}

fn dirs_base(pkg_name: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".local/fpm/overlay").join(pkg_name)
}
