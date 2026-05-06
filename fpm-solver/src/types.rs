use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct Package(pub String);

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    pub fn parse(s: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = s.splitn(3, '.').collect();
        let major = parts.first().unwrap_or(&"0").parse().unwrap_or(0);
        let minor = parts.get(1).unwrap_or(&"0").parse().unwrap_or(0);
        let patch = parts.get(2).map(|p| p.split('-').next().unwrap_or("0")).unwrap_or("0").parse().unwrap_or(0);
        Ok(Self { major, minor, patch })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct VersionReq {
    pub op: Op,
    pub version: Version,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Op {
    Gte,
    Gt,
    Lte,
    Lt,
    Eq,
    Any,
}

impl VersionReq {
    pub fn any() -> Self {
        Self { op: Op::Any, version: Version::new(0, 0, 0) }
    }

    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if s.is_empty() {
            return Self::any();
        }
        let (op, ver_str) = if let Some(rest) = s.strip_prefix(">=") {
            (Op::Gte, rest.trim())
        } else if let Some(rest) = s.strip_prefix("> ") {
            (Op::Gt, rest.trim())
        } else if let Some(rest) = s.strip_prefix("<=") {
            (Op::Lte, rest.trim())
        } else if let Some(rest) = s.strip_prefix("< ") {
            (Op::Lt, rest.trim())
        } else if let Some(rest) = s.strip_prefix('=') {
            (Op::Eq, rest.trim())
        } else {
            (Op::Gte, s)
        };
        let version = Version::parse(ver_str).unwrap_or(Version::new(0, 0, 0));
        Self { op, version }
    }

    pub fn matches(&self, v: &Version) -> bool {
        match self.op {
            Op::Any => true,
            Op::Gte => v >= &self.version,
            Op::Gt  => v > &self.version,
            Op::Lte => v <= &self.version,
            Op::Lt  => v < &self.version,
            Op::Eq  => v == &self.version,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Dep {
    pub name: String,
    pub req: VersionReq,
    pub optional: bool,
    pub reason: Option<String>,
}

impl Dep {
    pub fn required(name: impl Into<String>, req: VersionReq) -> Self {
        Self { name: name.into(), req, optional: false, reason: None }
    }
}
